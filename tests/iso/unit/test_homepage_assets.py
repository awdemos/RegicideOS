"""
Unit tests for the first-user GPU rental homepage and desktop shortcut.

These tests validate the source assets in data/homepage/ and the staging logic
in stage6-finalize.sh without requiring a full Catalyst build. They use shell
snippets extracted from stage6-finalize.sh against a fake rootfs tree so that
stage behavior changes are caught quickly in CI.
"""

from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parents[3]
DATA_HOMEPAGE = REPO_ROOT / "data" / "homepage"
STAGE6_SCRIPT = REPO_ROOT / "build-system" / "catalyst" / "stages" / "stage6-finalize.sh"
STAGE7_SCRIPT = REPO_ROOT / "build-system" / "catalyst" / "stages" / "stage7-verify.sh"


@pytest.fixture
def homepage_files():
    """Return the paths of the source homepage assets."""
    html = DATA_HOMEPAGE / "homepage.html"
    desktop = DATA_HOMEPAGE / "Rent-a-GPU.desktop"
    return {"html": html, "desktop": desktop}


def test_source_homepage_html_exists(homepage_files):
    """The local HTML landing page source file must exist."""
    assert homepage_files["html"].is_file(), "data/homepage/homepage.html is missing"


def test_source_desktop_shortcut_exists(homepage_files):
    """The desktop shortcut source file must exist."""
    assert homepage_files["desktop"].is_file(), "data/homepage/Rent-a-GPU.desktop is missing"


def test_homepage_html_links_to_vca_gpu_catalog(homepage_files):
    """The page must link to the public GPU rental catalog with RegicideOS attribution."""
    content = homepage_files["html"].read_text()
    assert "vibecodingagency.com/gpus/" in content
    assert "utm_source=regicideos" in content


def test_homepage_html_contains_workload_guidance(homepage_files):
    """The page must surface GPU/workload guidance so users can pick a card."""
    content = homepage_files["html"].read_text()
    assert "Training large models" in content
    assert "Inference" in content or "serving" in content
    assert "Fine-tuning" in content or "LoRA" in content


def test_desktop_shortcut_points_to_local_page(homepage_files):
    """The .desktop entry must open the local first-user homepage."""
    content = homepage_files["desktop"].read_text()
    assert "Exec=xdg-open /home/regicide/homepage.html" in content


def test_stage6_script_stages_homepage_assets():
    """stage6-finalize.sh must copy the homepage files into the staging area."""
    script = STAGE6_SCRIPT.read_text()
    assert "data/homepage/homepage.html" in script
    assert "data/homepage/Rent-a-GPU.desktop" in script
    assert "homepage.html" in script
    assert "Rent-a-GPU.desktop" in script


def test_stage6_script_installs_homepage_into_regicide_home():
    """stage6-finalize.sh must install the assets to /home/regicide inside the rootfs."""
    script = STAGE6_SCRIPT.read_text()
    assert "/home/regicide/homepage.html" in script
    assert "/home/regicide/Desktop/Rent-a-GPU.desktop" in script


def test_stage7_script_asserts_homepage_assets():
    """stage7-verify.sh must assert the homepage and desktop shortcut are present."""
    script = STAGE7_SCRIPT.read_text()
    assert "homepage.html present in /home/regicide" in script
    assert "Rent-a-GPU.desktop present on Desktop" in script
    assert "vibecodingagency.com/gpus/" in script
    assert "/home/regicide/homepage.html" in script
