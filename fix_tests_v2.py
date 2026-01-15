import os

def fix_file(path):
    with open(path, 'r') as f:
        content = f.read()
    
    # We want to replace (bump|module).alloc(...) with ExprRef::new((bump|module).alloc(...))
    
    pos = 0
    new_content = ""
    while True:
        # Find either "bump.alloc(" or "module.alloc("
        m1 = content.find("bump.alloc(", pos)
        m2 = content.find("module.alloc(", pos)
        
        if m1 == -1 and m2 == -1:
            new_content += content[pos:]
            break
        
        if m1 == -1: start = m2
        elif m2 == -1: start = m1
        else: start = min(m1, m2)
        
        # Check if already wrapped
        if start >= 13 and content[start-13:start] == "ExprRef::new(":
             new_content += content[pos:start+1]
             pos = start + 1
             continue

        new_content += content[pos:start]
        new_content += "ExprRef::new("
        
        # Find the matching closing parenthesis for this alloc(
        # We start at the parenthesis after 'alloc'
        p_start = content.find("(", start)
        paren_count = 1
        i = p_start + 1
        while i < len(content) and paren_count > 0:
            if content[i] == '(':
                paren_count += 1
            elif content[i] == ')':
                paren_count -= 1
            i += 1
        
        new_content += content[start:i]
        new_content += ")"
        pos = i

    if new_content != content:
        with open(path, 'w') as f:
            f.write(new_content)
        return True
    return False

# First, undo the mess from the previous script by removing ExprRef::new( if it's orphaned
# Actually, it's easier to just run a script that identifies and fixes the balance.

# Let's just try to fix the current broken files.
for root, dirs, files in os.walk('/home/mihailik/binaryen-fork/rust/binaryen-ir/src'):
    for file in files:
        if file.endswith('.rs'):
            fix_file(os.path.join(root, file))
