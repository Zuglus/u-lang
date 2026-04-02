import re

with open('type_checker.rs', 'r') as f:
    lines = f.readlines()

result = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Check for TypeError { (not struct definition, not impl)
    if ('TypeError {' in line and 
        'pub struct' not in line and 
        'impl TypeError' not in lines[max(0, i-1)]):
        
        # Collect the block
        block_lines = [line]
        j = i + 1
        brace_count = line.count('{') - line.count('}')
        
        while j < len(lines) and brace_count > 0:
            block_lines.append(lines[j])
            brace_count += lines[j].count('{') - lines[j].count('}')
            j += 1
        
        # Check if block already has context
        block_text = ''.join(block_lines)
        if 'context:' not in block_text:
            # Add context and help before closing brace
            # Find line with } and insert before it
            for k in range(len(block_lines) - 1, -1, -1):
                if block_lines[k].strip() == '}':
                    indent = len(block_lines[k-1]) - len(block_lines[k-1].lstrip())
                    block_lines.insert(k, ' ' * indent + 'context: None,\n')
                    block_lines.insert(k + 1, ' ' * indent + 'help: None,\n')
                    break
        
        result.extend(block_lines)
        i = j
    else:
        result.append(line)
        i += 1

with open('type_checker.rs', 'w') as f:
    f.writelines(result)

print('Done')
