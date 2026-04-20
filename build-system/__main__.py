"""Entry point for regicide-image-builder CLI.

Usage: python -m build_system
"""

import runpy
import sys
from pathlib import Path

# Ensure the build-system directory is on the path so the module resolves
sys.path.insert(0, str(Path(__file__).parent))

from regicide_image_builder import app

if __name__ == "__main__":
    app()
