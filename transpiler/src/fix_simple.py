with open('type_checker.rs', 'r') as f:
    lines = f.readlines()

result = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Check for TypeError { start
    if 'TypeError {' in line and 'pub struct' not in line and i > 0 and 'impl TypeError' not in lines[i-1]:
        # Collect multiline block
        block = [line]
        j = i + 1
        while j < len(lines) and '}' not in lines[j]:
            block.append(lines[j])
            j += 1
        if j < len(lines):
            block.append(lines[j])  # Add closing brace line
        
        block_text = ''.join(block)
        
        # Check if already has context or help (skip those)
        if 'context:' in block_text or 'help:' in block_text:
            result.extend(block)
            i = j + 1
            continue
        
        # Extract message
        msg_start = block_text.find('message:') + len('message:')
        msg_end = block_text.find(',\n', msg_start)
        message = block_text[msg_start:msg_end].strip()
        
        # Extract span  
        span_start = block_text.find('span:') + len('span:')
        span_end = block_text.find('\n', span_start)
        span = block_text[span_start:span_end].strip().rstrip(',')
        
        # Create new format
        indent = len(line) - len(line.lstrip())
        new_lines = [
            ' ' * indent + f'TypeError::new({message}, {span})\n'
        ]
        result.extend(new_lines)
        i = j + 1
    else:
        result.append(line)
        i += 1

with open('type_checker.rs', 'w') as f:
    f.writelines(result)

print('Done')
