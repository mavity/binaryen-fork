import re
import os

def fix_allocs(file_path):
    with open(file_path, 'r') as f:
        content = f.read()

    # Pattern for bump.alloc(Expression { ... })
    # We look for (bump|module).alloc(Expression { and find the matching }
    
    pos = 0
    new_content = ""
    while True:
        match = re.search(r'(bump|module)\.alloc\(Expression\s*\{', content[pos:])
        if not match:
            new_content += content[pos:]
            break
        
        start = pos + match.start()
        new_content += content[pos:start]
        
        # Find matching brace
        brace_count = 1
        i = start + match.end()
        while i < len(content) and brace_count > 0:
            if content[i] == '{':
                brace_count += 1
            elif content[i] == '}':
                brace_count -= 1
            i += 1
        
        # We found Expression { ... }
        # Now find the closing parenthesis of alloc(
        p_match = re.search(r'\)', content[i:])
        if p_match:
            end = i + p_match.end()
            alloc_call = content[start:end]
            new_content += f"ExprRef::new({alloc_call})"
            pos = end
        else:
            # Fallback
            new_content += content[start:start+1]
            pos = start + 1

    if new_content != content:
        with open(file_path, 'w') as f:
            f.write(new_content)
        return True
    return False

for root, dirs, files in os.walk('/home/mihailik/binaryen-fork/rust/binaryen-ir/src'):
    for file in files:
        if file.endswith('.rs'):
            path = os.path.join(root, file)
            if fix_allocs(path):
                print(f"Fixed {path}")
