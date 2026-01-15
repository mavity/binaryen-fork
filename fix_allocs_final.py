import os
import re

def fix_file(path):
    with open(path, 'r') as f:
        content = f.read()
    
    # regex for bump.alloc or module.alloc
    # we use a loop to handle overlaps if any (though there shouldn't be)
    
    changed = False
    
    # We want to wrap ANY (bump|module).alloc(...) with ExprRef::new(...)
    # BUT we must be careful not to wrap if it's already wrapped.
    
    # A safer way: find all (bump|module).alloc( and find their ranges.
    
    alloc_calls = []
    for m in re.finditer(r'(bump|module)\.alloc\(', content):
        start = m.start()
        # count parens from here
        p_start = content.find('(', start)
        paren_count = 1
        i = p_start + 1
        while i < len(content) and paren_count > 0:
            if content[i] == '(':
                paren_count += 1
            elif content[i] == ')':
                paren_count -= 1
            i += 1
        end = i
        
        # Check if it's already wrapped
        is_wrapped = False
        prefix = content[max(0, start-13):start]
        if "ExprRef::new(" in prefix:
            is_wrapped = True
            
        if not is_wrapped:
            alloc_calls.append((start, end))
            
    # Apply changes in reverse order
    for start, end in reversed(alloc_calls):
        content = content[:start] + "ExprRef::new(" + content[start:end] + ")" + content[end:]
        changed = True

    if changed:
        with open(path, 'w') as f:
            f.write(content)
        return True
    return False

for root, dirs, files in os.walk('/home/mihailik/binaryen-fork/rust/binaryen-ir/src'):
    for file in files:
        if file.endswith('.rs'):
            if fix_file(os.path.join(root, file)):
                print(f"Fixed {file}")
