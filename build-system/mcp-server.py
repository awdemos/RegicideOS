#!/usr/bin/env python3
"""MCP server for RegicideOS build observability.

Exposes build status, stage logs, and artifacts as MCP resources, plus tools
for querying and launching builds.

Install the Python MCP SDK before running:
    pip install mcp

Run with stdio transport for OpenCode-style MCP clients:
    python build-system/mcp-server.py
"""

from __future__ import annotations

import json
import os
import subprocess
from pathlib import Path
from typing import Any

from mcp.server import Server
from mcp.server.stdio import stdio_server
from mcp.types import Resource, TextContent, Tool


REPO_ROOT = Path(__file__).resolve().parent.parent
BUILD_DIR = REPO_ROOT / "build-system" / "catalyst" / "output"
STATUS_FILE = BUILD_DIR / "build-status.jsonl"
ARTIFACTS = {
    "stage4-tarball": BUILD_DIR / "stage4-amd64-systemd-cosmic.tar.xz",
    "squashfs": REPO_ROOT / "regicide-cosmic.img",
    "unencrypted-qcow2": BUILD_DIR / "regicide-cosmic.qcow2",
    "encrypted-qcow2": BUILD_DIR / "regicide-cosmic-enc.qcow2",
}

app = Server("regicide-build")


def _read_status_lines(limit: int = 0) -> list[dict[str, Any]]:
    if not STATUS_FILE.exists():
        return []
    lines = STATUS_FILE.read_text().strip().splitlines()
    if limit > 0:
        lines = lines[-limit:]
    return [json.loads(line) for line in lines if line.strip()]


def _latest_stage_event(stage: str) -> dict[str, Any] | None:
    for entry in reversed(_read_status_lines()):
        if entry.get("stage") == stage:
            return entry
    return None


def _artifacts_info() -> dict[str, Any]:
    info: dict[str, Any] = {}
    for name, path in ARTIFACTS.items():
        if path.exists():
            info[name] = {
                "path": str(path),
                "size": path.stat().st_size,
                "mtime": path.stat().st_mtime,
            }
        else:
            info[name] = {"path": str(path), "exists": False}
    return info


@app.list_resources()
async def list_resources() -> list[Resource]:
    return [
        Resource(
            uri="regicide://build/status",
            name="RegicideOS Build Status",
            mimeType="application/json",
            description="Latest build status across all stages",
        ),
        Resource(
            uri="regicide://build/log",
            name="RegicideOS Build Log",
            mimeType="application/x-ndjson",
            description="Full build-status.jsonl log",
        ),
        Resource(
            uri="regicide://build/artifacts",
            name="RegicideOS Build Artifacts",
            mimeType="application/json",
            description="Available build output files",
        ),
    ]


@app.read_resource()
async def read_resource(uri: str) -> str:
    if uri == "regicide://build/status":
        entries = _read_status_lines()
        if not entries:
            return json.dumps({"status": "no build status recorded"})
        latest = entries[-1]
        stages = sorted({e["stage"] for e in entries})
        return json.dumps(
            {
                "latest": latest,
                "stages_seen": stages,
                "completed_stages": [
                    s
                    for s in stages
                    if any(e["stage"] == s and e["event"] == "complete" for e in entries)
                ],
            },
            indent=2,
        )
    if uri == "regicide://build/log":
        if not STATUS_FILE.exists():
            return ""
        return STATUS_FILE.read_text()
    if uri == "regicide://build/artifacts":
        return json.dumps(_artifacts_info(), indent=2)
    raise ValueError(f"Unknown resource: {uri}")


@app.list_tools()
async def list_tools() -> list[Tool]:
    return [
        Tool(
            name="get_stage_status",
            description="Return the most recent event for a build stage",
            inputSchema={
                "type": "object",
                "properties": {
                    "stage": {
                        "type": "string",
                        "description": "Stage name, e.g. stage4-cosmic",
                    }
                },
                "required": ["stage"],
            },
        ),
        Tool(
            name="list_artifacts",
            description="List available RegicideOS build artifacts",
            inputSchema={
                "type": "object",
                "properties": {},
            },
        ),
        Tool(
            name="start_build",
            description="Start the Dagger build pipeline in the background",
            inputSchema={
                "type": "object",
                "properties": {
                    "plain": {
                        "type": "boolean",
                        "description": "Use plain Dagger progress output",
                        "default": True,
                    },
                },
            },
        ),
    ]


@app.call_tool()
async def call_tool(name: str, arguments: dict[str, Any]) -> list[TextContent]:
    if name == "get_stage_status":
        stage = arguments["stage"]
        event = _latest_stage_event(stage)
        return [TextContent(type="text", text=json.dumps(event, indent=2))]
    if name == "list_artifacts":
        return [TextContent(type="text", text=json.dumps(_artifacts_info(), indent=2))]
    if name == "start_build":
        plain = arguments.get("plain", True)
        env = os.environ.copy()
        if plain:
            env["DAGGER_PROGRESS"] = "plain"
        cmd = ["dagger", "run", "python", "build-system/dagger_pipeline.py"]
        if plain:
            cmd.append("--plain")
        proc = subprocess.Popen(
            cmd,
            cwd=str(REPO_ROOT),
            env=env,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
        return [
            TextContent(
                type="text",
                text=json.dumps(
                    {"started": True, "pid": proc.pid, "command": " ".join(cmd)}
                ),
            )
        ]
    raise ValueError(f"Unknown tool: {name}")


async def main() -> None:
    async with stdio_server() as (read_stream, write_stream):
        await app.run(
            read_stream,
            write_stream,
            app.create_initialization_options(),
        )


if __name__ == "__main__":
    import asyncio

    asyncio.run(main())
