import re

with open('type_checker.rs', 'r') as f:
    content = f.read()

# Pattern 1: .ok_or_else(|| TypeError { message: ..., span: ... })
pattern1 = r'\.ok_or_else\(\|\| TypeError \{\s*message:\s*([^}]+),\s*span:\s*([^}]+)\s*\}\)'
replacement1 = r'.ok_or_else(|| TypeError::new(\1, \2))'
content = re.sub(pattern1, replacement1, content, flags=re.DOTALL)

# Pattern 2: Err(TypeError { message: ..., span: ... })
pattern2 = r'Err\(TypeError \{\s*message:\s*([^}]+),\s*span:\s*([^}]+)\s*\}\)'
replacement2 = r'Err(TypeError::new(\1, \2))'
content = re.sub(pattern2, replacement2, content, flags=re.DOTALL)

# Pattern 3: return Err(TypeError { message: ..., span: ... })
pattern3 = r'return Err\(TypeError \{\s*message:\s*([^}]+),\s*span:\s*([^}]+)\s*\}\)'
replacement3 = r'return Err(TypeError::new(\1, \2))'
content = re.sub(pattern3, replacement3, content, flags=re.DOTALL)

with open('type_checker.rs', 'w') as f:
    f.write(content)

print('Done')
