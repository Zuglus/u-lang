import re

with open('type_checker.rs', 'r') as f:
    content = f.read()

# Pattern to match TypeError { ... } blocks
pattern = r'TypeError\s*\{([^}]+)\}'

def replace_typeerror(match):
    inner = match.group(1)
    # Check if already converted or has ..Default::default()
    if 'context' in inner and 'help' in inner:
        # Already has all fields - just add Default if needed
        return match.group(0)
    
    # Extract message and span
    msg_match = re.search(r'message:\s*([^,]+(?:\([^)]+\)[^,]*)*),', inner)
    span_match = re.search(r'span:\s*([^,]+),', inner)
    
    if msg_match and span_match:
        msg = msg_match.group(1).strip()
        span = span_match.group(1).strip()
        return f'TypeError::new({msg}, {span})'
    
    return match.group(0)

content = re.sub(pattern, replace_typeerror, content, flags=re.DOTALL)

with open('type_checker.rs', 'w') as f:
    f.write(content)

print('Done')
