import os

def fix_balance(path):
    with open(path, 'r') as f:
        content = f.read()
    
    pos = 0
    new_content = ""
    while True:
        target = "ExprRef::new("
        start = content.find(target, pos)
        if start == -1:
            new_content += content[pos:]
            break
        
        # If it matches "ExprRef::new(", we expect a matching ")" later.
        # But we only want to fix those where we have an inner call like "alloc("
        alloc_pos = content.find(".alloc(", start)
        if alloc_pos == -1 or alloc_pos > start + 50: # arbitrary limit to ensure it's nearby
            new_content += content[pos:start + len(target)]
            pos = start + len(target)
            continue
            
        new_content += content[pos:start + len(target)]
        
        # Find the end of the .alloc(...) call
        p_start = content.find("(", alloc_pos)
        paren_count = 1
        i = p_start + 1
        while i < len(content) and paren_count > 0:
            if content[i] == '(':
                paren_count += 1
            elif content[i] == ')':
                paren_count -= 1
            i += 1
        
        # We found the inner call: content[start+12:i] (roughly)
        new_content += content[start + len(target):i]
        
        # Now check if it's followed by a ')'
        # skip whitespace
        j = i
        while j < len(content) and content[j].isspace():
            j += 1
        
        if j < len(content) and content[j] == ')':
            # Already has one, skip
            new_content += content[i:j+1]
            pos = j + 1
        else:
            # Add the missing one
            new_content += ")"
            pos = i

    if new_content != content:
        with open(path, 'w') as f:
            f.write(new_content)
        return True
    return False

for root, dirs, files in os.walk('/home/mihailik/binaryen-fork/rust/binaryen-ir/src'):
    for file in files:
        if file.endswith('.rs'):
            if fix_balance(os.path.join(root, file)):
                print(f"Balanced {file}")
