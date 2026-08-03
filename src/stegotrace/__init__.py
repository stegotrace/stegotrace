"""StegoTrace public package."""

__version__ = "0.2.0"

from .engine import analyze_file, extract_artifact

__all__ = ["analyze_file", "extract_artifact"]
