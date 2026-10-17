import json

def fix_errors():
    with open("errors.json", "r") as f:
        lines = f.read().splitlines()
        
    fixes = []
    print(f"Got {len(lines)} JSON messages from errors.json")
    for line in lines:
        try:
            msg = json.loads(line)
        except:
            continue
        if msg.get("reason") == "compiler-message" and msg["message"].get("code") and msg["message"]["code"]["code"] == "E0609":
            err_msg = msg["message"]["message"]
            if "selection::Region" in err_msg or "Region" in err_msg:
                for span in msg["message"]["spans"]:
                    if span["is_primary"]:
                        try:
                            field = err_msg.split("`")[1]
                        except:
                            continue
                        fixes.append({
                            "file": span["file_name"],
                            "line": span["line_start"],
                            "col_start": span["column_start"],
                            "col_end": span["column_end"],
                            "field": field
                        })
    
    # E0560 is "struct has no field named `x`" (e.g. `Region { x: 0 }`)
    for line in lines:
        try:
            msg = json.loads(line)
        except:
            continue
        if msg.get("reason") == "compiler-message" and msg["message"].get("code") and msg["message"]["code"]["code"] == "E0560":
            err_msg = msg["message"]["message"]
            # "struct `Region` has no field named `x`"
            if "Region" in err_msg:
                for span in msg["message"]["spans"]:
                    if span["is_primary"]:
                        try:
                            # It says struct `Region` has no field named `x`
                            # Just extract what is inside the span text!
                            field = span["text"][0]["text"][span["column_start"]-1 : span["column_end"]-1]
                        except Exception as e:
                            print(f"Error {e}")
                            continue
                        
                        # Sometimes struct definition has field x:
                        fixes.append({
                            "file": span["file_name"],
                            "line": span["line_start"],
                            "col_start": span["column_start"],
                            "col_end": span["column_end"],
                            "field": field
                        })

    # Remove duplicates
    unique_fixes = { (f["file"], f["line"], f["col_start"]): f for f in fixes }.values()
    fixes = list(unique_fixes)

    # Sort backwards per file
    fixes.sort(key=lambda x: (x["file"], x["line"], x["col_start"]), reverse=True)
    
    for fix in fixes:
        filename = fix["file"]
        with open(filename, "r") as f:
            content = f.read().splitlines()
        
        line_idx = fix["line"] - 1
        line_str = content[line_idx]
        col_start = fix["col_start"] - 1
        col_end = fix["col_end"] - 1
        
        old_field = fix["field"]
        
        # fallback parsing for E0560
        if ":" in old_field:
            old_field = old_field.split(":")[0].strip()
        
        new_field = f"physical_{old_field}"
        
        # find the exact 'old_field' in that area
        # often span is accurate for E0609, but for E0560 it might include the colon.
        # let's be safe.
        
        # Check if the substring at the location is exactly old_field
        if line_str[col_start:col_end] == old_field:
            content[line_idx] = line_str[:col_start] + new_field + line_str[col_end:]
        elif old_field in line_str[col_start:col_end]:
            # if `x:` was matched, replace just `x`
            sub = line_str[col_start:col_end]
            sub = sub.replace(old_field, new_field, 1)
            content[line_idx] = line_str[:col_start] + sub + line_str[col_end:]
        elif old_field in line_str:
            print(f"Warning: Expected '{old_field}' but found '{line_str[col_start:col_end]}' at {filename}:{fix['line']} -> falling back to simplistic replace")
            content[line_idx] = line_str.replace(f"{old_field}:", f"{new_field}:") # For struct literal
        else:
            print(f"Warning: Could not apply {old_field}->{new_field} at {filename}:{fix['line']}")

        with open(filename, "w") as f:
            f.write("\n".join(content) + "\n")
            
    print(f"Applied {len(fixes)} fixes")

if __name__ == "__main__":
    fix_errors()
