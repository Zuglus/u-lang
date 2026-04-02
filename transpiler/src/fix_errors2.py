import re

with open('type_checker.rs', 'r') as f:
    content = f.read()

# Pattern for ok_or_else and other closures
pattern = r'TypeError\s*\{\s*message:\s*([^,]+),\s*span:\s*([^,]+?),\s*\}'
replacement = r'TypeError::new(\1, \2)'
content = re.sub(pattern, replacement, content)

with open('type_checker.rs', 'w') as f:
    f.write(content)

print("Done")
