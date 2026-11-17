"""Focused tests for the dependency-free retrieval-corpus validator."""

from __future__ import annotations

import contextlib
import io
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from wiki import lint as rag_lint


class KnowledgeValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name).resolve()
        self.wiki_root = self.root / "wiki"
        self.pages_root = self.wiki_root / "pages"
        self.pages_root.mkdir(parents=True)
        (self.root / "src").mkdir()
        (self.root / "src" / "lib.rs").write_text("// canonical source\n", encoding="utf-8")
        self.index_path = self.wiki_root / "index.md"
        self.index_path.write_text("[Example](pages/example.md)\n", encoding="utf-8")
        self.constants = mock.patch.multiple(
            rag_lint,
            REPO_ROOT=self.root,
            WIKI_ROOT=self.wiki_root,
            PAGES_ROOT=self.pages_root,
            INDEX_PATH=self.index_path,
        )
        self.constants.start()

    def tearDown(self) -> None:
        self.constants.stop()
        self.temporary_directory.cleanup()

    def write_page(
        self,
        *,
        include_summary: bool = True,
        source_path: str = "src/lib.rs",
        body_suffix: str = "",
    ) -> Path:
        metadata = [
            "---",
            "id: concept.example",
            "kind: concept",
            "title: Example",
        ]
        if include_summary:
            metadata.append(
                "summary: Explains enough project-specific detail to form a valid retrieval summary."
            )
        metadata.extend(
            [
                "tags:",
                "  - example",
                "  - retrieval",
                "source_paths:",
                f"  - {source_path}",
                "related:",
                "  - concept.example",
                "---",
                "# Example",
                "",
                "Valid body.",
            ]
        )
        if body_suffix:
            metadata.extend(["", body_suffix])
        page = self.pages_root / "example.md"
        page.write_text("\n".join(metadata) + "\n", encoding="utf-8")
        return page

    def test_invalid_metadata_is_reported(self) -> None:
        page = self.write_page(include_summary=False)
        errors: list[str] = []

        rag_lint.validate_page(page, errors)

        self.assertTrue(any("missing fields: summary" in error for error in errors))

    def test_broken_local_link_is_reported(self) -> None:
        page = self.write_page(body_suffix="[Missing](missing.md)")
        errors: list[str] = []

        rag_lint.validate_page(page, errors)

        self.assertTrue(any("local Markdown link does not exist" in error for error in errors))

    def test_missing_source_path_is_reported(self) -> None:
        page = self.write_page(source_path="src/missing.rs")
        errors: list[str] = []

        rag_lint.validate_page(page, errors)

        self.assertTrue(any("source path does not name a file" in error for error in errors))

    def test_diagnostics_are_deterministic_and_main_fails(self) -> None:
        self.write_page(source_path="src/missing.rs", body_suffix="[Missing](missing.md)")

        first = rag_lint.validate()
        second = rag_lint.validate()

        self.assertEqual(first, second)
        with mock.patch.object(rag_lint, "validate", return_value=first):
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                exit_code = rag_lint.main()
        self.assertEqual(exit_code, 1)
        self.assertIn("RAG knowledge-base validation failed", output.getvalue())


if __name__ == "__main__":
    unittest.main()
