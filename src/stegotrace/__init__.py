"""StegoTrace public package."""

from .engine import analyze_file, extract_artifact

__all__ = ["analyze_file", "extract_artifact"]
__version__ = "0.2.0"
