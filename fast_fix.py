import re

files = ['src/selection.rs', 'tests/bench_functions.rs']

for filename in files:
    with open(filename, 'r') as f:
        content = f.read()

    # Field accesses on known variables representing regions
    vars = r'(region|r|local|copied|cloned|Region|Region \{)'
    
    # We replace .x -> .physical_x etc
    content = re.sub(r'\b(region|r|local|copied|cloned)\.x\b', r'\1.physical_x', content)
    content = re.sub(r'\b(region|r|local|copied|cloned)\.y\b', r'\1.physical_y', content)
    content = re.sub(r'\b(region|r|local|copied|cloned)\.width\b', r'\1.physical_width', content)
    content = re.sub(r'\b(region|r|local|copied|cloned)\.height\b', r'\1.physical_height', content)

    # Struct instantiations Region { x: ..., y: ... }
    # To be safe, we'll replace `x: ` with `physical_x: ` only inside `Region {` blocks.
    # However, since we can't easily parse matching braces with simple regex, let's just globally replace
    # \bx:\s with physical_x: and carefully avoid known false positives.
    # Looking at the code, mostly `x:` is used for Region.
    # Let's replace `\bx:` if the line is part of Region or just do it.
    
    # Actually, a better option is to do it line-by-line and if we see Region { or we are inside Region tests
    
    new_lines = []
    in_region = False
    for line in content.splitlines():
        if "Region" in line and "{" in line:
            in_region = True
        
        if in_region or "x:" in line or "y:" in line or "width" in line or "height" in line:
            # We want to replace x: 0 with physical_x: 0, x, with physical_x,
            line = re.sub(r'\bx:\s*([^,]+)', r'physical_x: \1', line)
            line = re.sub(r'\by:\s*([^,]+)', r'physical_y: \1', line)
            line = re.sub(r'\bwidth:\s*([^,]+)', r'physical_width: \1', line)
            line = re.sub(r'\bheight:\s*([^,]+)', r'physical_height: \1', line)
            line = re.sub(r'\bwidth,', r'physical_width: width,', line)
            line = re.sub(r'\bheight,', r'physical_height: height,', line)
            
        new_lines.append(line)
        
        if "}" in line and in_region:
            in_region = False
            
    with open(filename, 'w') as f:
        f.write("\n".join(new_lines) + "\n")

print("Fixed")
