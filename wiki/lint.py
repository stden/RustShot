import os
import re

wiki_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)))
index_file = os.path.join(wiki_dir, "index.md")

with open(index_file, "r", encoding="utf-8") as f:
    index_content = f.read()

# Extract links from index
links = set(re.findall(r'\]\((pages/[^)]+)\)', index_content))

# Find orphaned pages
pages_dir = os.path.join(wiki_dir, "pages")
orphans = []
for filename in os.listdir(pages_dir):
    if filename.endswith(".md"):
        rel_path = f"pages/{filename}"
        if rel_path not in links:
            orphans.append(rel_path)

if orphans:
    print("Orphaned pages found (not in index.md):")
    for o in orphans:
        print(" -", o)
else:
    print("No orphaned pages found!")
