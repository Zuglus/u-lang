with open('type_checker.rs', 'r') as f:
    lines = f.readlines()

result = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Check if this line contains TypeError { without context
    if 'TypeError {' in line and 'context' not in lines[min(i+1, len(lines)-1)]:
        # Find all lines until the closing brace
        block = [line]
        j = i + 1
        brace_count = line.count('{') - line.count('}')
        
        while j < len(lines) and brace_count > 0:
            block.append(lines[j])
            brace_count += lines[j].count('{') - lines[j].count('}')
            j += 1
        
        # Check if block already has context/help
        block_text = ''.join(block)
        if 'context:' not in block_text:
            # Find the last line with content (before closing brace)
            for k in range(len(block) - 1, -1, -1):
                if block[k].strip() and block[k].strip() != '}':
                    # Insert context and help before the closing brace
                    indent = len(block[k]) - len(block[k].lstrip())
                    block.insert(k + 1, ' ' * indent + 'context: None,\n')
                    block.insert(k + 2, ' ' * indent + 'help: None,\n')
                    break
        
        result.extend(block)
        i = j
    else:
        result.append(line)
        i += 1

with open('type_checker.rs', 'w') as f:
    f.writelines(result)

print('Done')
