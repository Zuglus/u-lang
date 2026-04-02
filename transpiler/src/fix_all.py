import re

with open('type_checker.rs', 'r') as f:
    content = f.read()

# Replace TypeError { message: ..., span: ... } with added context and help
# Pattern matches multiline TypeError blocks
pattern = r'TypeError\s*\{\s*message:\s*(.+?),\s*span:\s*(.+?)\s*\}'

def replace_match(m):
    msg = m.group(1).strip()
    span = m.group(2).strip()
    return f'TypeError {{\n                    message: {msg},\n                    span: {span},\n                    context: None,\n                    help: None,\n                }}'

content = re.sub(pattern, replace_match, content, flags=re.DOTALL)

with open('type_checker.rs', 'w') as f:
    f.write(content)

print('Done')
