"""Validate the RustShot retrieval corpus without third-party dependencies."""

from __future__ import annotations

import re
import sys
from dataclasses import dataclass
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parent.parent
WIKI_ROOT = REPO_ROOT / "wiki"
PAGES_ROOT = WIKI_ROOT / "pages"
INDEX_PATH = WIKI_ROOT / "index.md"
REQUIRED_FIELDS = {
    "id",
    "kind",
    "title",
    "summary",
    "tags",
    "source_paths",
    "related",
}
LIST_FIELDS = {"tags", "source_paths", "related"}
VALID_KINDS = {"concept", "entity", "source"}
ID_PATTERN = re.compile(r"^(concept|entity|source)\.[a-z0-9]+(?:-[a-z0-9]+)*$")
TAG_PATTERN = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")
MARKDOWN_LINK_PATTERN = re.compile(r"\[[^]]*\]\(([^)]+)\)")


@dataclass(frozen=True)
class Page:
    path: Path
    metadata: dict[str, str | list[str]]
    body: str


def parse_front_matter(path: Path) -> tuple[dict[str, str | list[str]], str]:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise ValueError("must start with YAML front matter")

    try:
        closing = lines.index("---", 1)
    except ValueError as error:
        raise ValueError("front matter has no closing '---'") from error

    metadata: dict[str, str | list[str]] = {}
    active_list: str | None = None
    for line_number, line in enumerate(lines[1:closing], start=2):
        if line.startswith("  - "):
            if active_list is None:
                raise ValueError(f"line {line_number}: list item has no field")
            value = line[4:].strip()
            if not value:
                raise ValueError(f"line {line_number}: list item is empty")
            values = metadata[active_list]
            assert isinstance(values, list)
            values.append(value)
            continue

        match = re.fullmatch(r"([a-z_]+):(?: (.*))?", line)
        if match is None:
            raise ValueError(f"line {line_number}: unsupported front matter syntax")
        key, scalar = match.groups()
        if key in metadata:
            raise ValueError(f"line {line_number}: duplicate field '{key}'")
        if key in LIST_FIELDS:
            if scalar:
                raise ValueError(f"line {line_number}: '{key}' must use a bullet list")
            metadata[key] = []
            active_list = key
        else:
            if not scalar:
                raise ValueError(f"line {line_number}: '{key}' must have a value")
            metadata[key] = scalar.strip()
            active_list = None

    body = "\n".join(lines[closing + 1 :]).strip()
    return metadata, body


def local_markdown_targets(page: Page) -> list[Path]:
    targets: list[Path] = []
    for raw_target in MARKDOWN_LINK_PATTERN.findall(page.body):
        target = raw_target.split("#", maxsplit=1)[0]
        if not target or re.match(r"^[a-z]+://", target):
            continue
        targets.append((page.path.parent / target).resolve())
    return targets


def validate_page(path: Path, errors: list[str]) -> Page | None:
    relative = path.relative_to(REPO_ROOT).as_posix()
    try:
        metadata, body = parse_front_matter(path)
    except ValueError as error:
        errors.append(f"{relative}: {error}")
        return None

    missing = REQUIRED_FIELDS - metadata.keys()
    extra = metadata.keys() - REQUIRED_FIELDS
    if missing:
        errors.append(f"{relative}: missing fields: {', '.join(sorted(missing))}")
    if extra:
        errors.append(f"{relative}: unknown fields: {', '.join(sorted(extra))}")

    page = Page(path=path, metadata=metadata, body=body)
    page_id = metadata.get("id")
    kind = metadata.get("kind")
    title = metadata.get("title")
    summary = metadata.get("summary")
    tags = metadata.get("tags")
    source_paths = metadata.get("source_paths")
    related = metadata.get("related")

    if not isinstance(page_id, str) or ID_PATTERN.fullmatch(page_id) is None:
        errors.append(f"{relative}: 'id' must be a stable kind.slug identifier")
    if kind not in VALID_KINDS:
        errors.append(f"{relative}: 'kind' must be concept, entity, or source")
    if isinstance(page_id, str) and isinstance(kind, str) and not page_id.startswith(f"{kind}."):
        errors.append(f"{relative}: id prefix must match kind '{kind}'")
    if not isinstance(title, str) or not title:
        errors.append(f"{relative}: 'title' must be non-empty")
    elif not body.startswith(f"# {title}\n"):
        errors.append(f"{relative}: first heading must be '# {title}'")
    if not isinstance(summary, str) or not 40 <= len(summary) <= 240:
        errors.append(f"{relative}: 'summary' must contain 40 to 240 characters")

    if not isinstance(tags, list) or len(tags) < 2:
        errors.append(f"{relative}: 'tags' must contain at least two entries")
    elif len(tags) != len(set(tags)) or any(TAG_PATTERN.fullmatch(tag) is None for tag in tags):
        errors.append(f"{relative}: tags must be unique lowercase slugs")

    if not isinstance(source_paths, list) or not source_paths:
        errors.append(f"{relative}: 'source_paths' must not be empty")
    else:
        if len(source_paths) != len(set(source_paths)):
            errors.append(f"{relative}: source paths must be unique")
        for source_path in source_paths:
            candidate = (REPO_ROOT / source_path).resolve()
            if candidate == REPO_ROOT or REPO_ROOT not in candidate.parents:
                errors.append(f"{relative}: source path escapes the repository: {source_path}")
            elif candidate == WIKI_ROOT or WIKI_ROOT in candidate.parents:
                errors.append(f"{relative}: source path must point outside the retrieval corpus: {source_path}")
            elif not candidate.is_file():
                errors.append(f"{relative}: source path does not name a file: {source_path}")

    if not isinstance(related, list) or not related:
        errors.append(f"{relative}: 'related' must not be empty")
    elif len(related) != len(set(related)):
        errors.append(f"{relative}: related ids must be unique")

    for target in local_markdown_targets(page):
        if target == REPO_ROOT or REPO_ROOT not in target.parents:
            errors.append(f"{relative}: local Markdown link escapes the repository: {target}")
        elif not target.exists():
            try:
                display_target = target.relative_to(REPO_ROOT).as_posix()
            except ValueError:
                display_target = str(target)
            errors.append(
                f"{relative}: local Markdown link does not exist: "
                f"{display_target}"
            )

    return page


def validate_index(page_paths: list[Path], errors: list[str]) -> None:
    index = INDEX_PATH.read_text(encoding="utf-8")
    linked = re.findall(r"\]\((pages/[^)#]+\.md)(?:#[^)]+)?\)", index)
    duplicates = sorted(path for path in set(linked) if linked.count(path) > 1)
    if duplicates:
        errors.append(f"wiki/index.md: duplicate page links: {', '.join(duplicates)}")

    expected = {path.relative_to(WIKI_ROOT).as_posix() for path in page_paths}
    actual = set(linked)
    missing = expected - actual
    unknown = actual - expected
    if missing:
        errors.append(f"wiki/index.md: unindexed pages: {', '.join(sorted(missing))}")
    if unknown:
        errors.append(f"wiki/index.md: links to missing pages: {', '.join(sorted(unknown))}")


def validate() -> list[str]:
    errors: list[str] = []
    page_paths = sorted(PAGES_ROOT.glob("*.md"))
    if not page_paths:
        errors.append("wiki/pages: retrieval corpus is empty")
    validate_index(page_paths, errors)

    pages = [page for path in page_paths if (page := validate_page(path, errors)) is not None]
    ids: dict[str, Path] = {}
    for page in pages:
        page_id = page.metadata.get("id")
        if not isinstance(page_id, str):
            continue
        if page_id in ids:
            first = ids[page_id].relative_to(REPO_ROOT).as_posix()
            second = page.path.relative_to(REPO_ROOT).as_posix()
            errors.append(f"duplicate id '{page_id}': {first}, {second}")
        ids[page_id] = page.path

    for page in pages:
        related = page.metadata.get("related")
        if not isinstance(related, list):
            continue
        relative = page.path.relative_to(REPO_ROOT).as_posix()
        for related_id in related:
            if related_id not in ids:
                errors.append(f"{relative}: related id does not exist: {related_id}")

    return errors


def main() -> int:
    errors = validate()
    if errors:
        print("RAG knowledge-base validation failed:")
        for error in errors:
            print(f" - {error}")
        return 1

    page_count = len(list(PAGES_ROOT.glob("*.md")))
    print(f"RAG knowledge base is valid: {page_count} indexed pages")
    return 0


if __name__ == "__main__":
    sys.exit(main())
