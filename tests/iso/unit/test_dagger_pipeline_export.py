"""
Unit tests for the chunked tarball export helper in build-system/dagger_pipeline.py.
"""

import asyncio
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import AsyncMock, MagicMock, patch

import importlib.util

import dagger

ROOT = Path(__file__).parent.parent.parent.parent


def _load_pipeline_module():
    spec = importlib.util.spec_from_file_location(
        "dagger_pipeline", str(ROOT / "build-system" / "dagger_pipeline.py")
    )
    module = importlib.util.module_from_spec(spec)
    sys.modules["dagger_pipeline"] = module
    spec.loader.exec_module(module)
    return module


_pipeline = _load_pipeline_module()
_export_file_with_retry = _pipeline._export_file_with_retry
_CHUNK_RETRIES = _pipeline._CHUNK_RETRIES
assemble_tarball_chunks = _pipeline.assemble_tarball_chunks
export_tarball_in_chunks = _pipeline.export_tarball_in_chunks


class FakeTransportError(dagger.TransportError):
    """Concrete transport error for tests; dagger.TransportError is abstract."""

    def __init__(self, message: str = "fake transport error"):
        super().__init__(message)


class TestAssembleTarballChunks(unittest.TestCase):
    """Tests for the host-side tarball reassembly helper."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())

    def tearDown(self):
        for path in self.tmpdir.rglob("*"):
            path.unlink()
        self.tmpdir.rmdir()

    def test_reassembles_chunks_in_order(self):
        chunks = [
            self.tmpdir / "chunk_0000",
            self.tmpdir / "chunk_0001",
            self.tmpdir / "chunk_0002",
        ]
        for i, chunk in enumerate(chunks):
            chunk.write_bytes(f"chunk{i}-data".encode())
        output = self.tmpdir / "stage4.tar.xz"

        assemble_tarball_chunks(chunks, output)

        self.assertEqual(output.read_bytes(), b"chunk0-datachunk1-datachunk2-data")


class TestExportFileWithRetry(unittest.TestCase):
    """Tests for the per-chunk export retry helper."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())

    def tearDown(self):
        for path in self.tmpdir.rglob("*"):
            path.unlink()
        self.tmpdir.rmdir()

    def _run(self, coro):
        return asyncio.run(coro)

    def test_succeeds_first_attempt(self):
        file = MagicMock()
        file.export = AsyncMock()
        dest = self.tmpdir / "out.tar"

        self._run(_export_file_with_retry(file, dest))

        file.export.assert_awaited_once_with(str(dest))

    def test_retries_then_succeeds(self):
        file = MagicMock()
        file.export = AsyncMock(side_effect=[FakeTransportError(), None])
        dest = self.tmpdir / "out.tar"

        with patch("asyncio.sleep", new=AsyncMock()):
            self._run(_export_file_with_retry(file, dest))

        self.assertEqual(file.export.await_count, 2)

    def test_raises_after_exhausting_retries(self):
        file = MagicMock()
        file.export = AsyncMock(side_effect=FakeTransportError())
        dest = self.tmpdir / "out.tar"

        with patch("asyncio.sleep", new=AsyncMock()):
            with self.assertRaises(dagger.TransportError):
                self._run(_export_file_with_retry(file, dest))

        self.assertEqual(file.export.await_count, _CHUNK_RETRIES)


class TestExportTarballInChunks(unittest.TestCase):
    """Tests for the high-level chunked export flow."""

    def setUp(self):
        self.tmpdir = Path(tempfile.mkdtemp())
        self.chunk_contents = {
            "stage4.tar.xz.chunk_0000": b"first",
            "stage4.tar.xz.chunk_0001": b"second",
            "stage4.tar.xz.chunk_0002": b"third",
        }

    def tearDown(self):
        for path in self.tmpdir.rglob("*"):
            path.unlink()
        self.tmpdir.rmdir()

    def _run(self, coro):
        return asyncio.run(coro)

    def _make_mock_container(self):
        container = MagicMock()
        split_container = MagicMock()
        container.with_exec.return_value = split_container
        dir_mock = MagicMock()
        dir_mock.entries = AsyncMock(return_value=list(self.chunk_contents.keys()))
        split_container.directory.return_value = dir_mock

        async def fake_export(dest: str):
            Path(dest).write_bytes(self.chunk_contents[Path(dest).name])

        def file_side_effect(path: str):
            file_mock = MagicMock()
            file_mock.export = AsyncMock(side_effect=fake_export)
            return file_mock

        split_container.file.side_effect = file_side_effect
        return container

    def test_exports_chunks_and_reassembles(self):
        container = self._make_mock_container()
        local_path = self.tmpdir / "stage4.tar.xz"

        result = self._run(
            export_tarball_in_chunks(
                container,
                "/src/build-system/catalyst/output/stage4-amd64-systemd-cosmic.tar.xz",
                local_path,
            )
        )

        self.assertEqual(result.resolve(), local_path.resolve())
        self.assertEqual(local_path.read_bytes(), b"firstsecondthird")
        for name in self.chunk_contents:
            self.assertFalse((self.tmpdir / name).exists())


if __name__ == "__main__":
    unittest.main()
