with open('type_checker.rs', 'r') as f:
    lines = f.readlines()

result = []
i = 0
while i < len(lines):
    line = lines[i]
    
    # Look for TypeError { that's not struct definition
    if 'TypeError {' in line and 'pub struct' not in lines[i-1] if i > 0 else True:
        if 'pub struct' not in line and 'impl TypeError' not in line:
            # Find the end of this TypeError block (closing })
            block_start = i
            j = i + 1
            brace_count = line.count('{') - line.count('}')
            
            while j < len(lines) and brace_count > 0:
                brace_count += lines[j].count('{') - lines[j].count('}')
                j += 1
            
            block_end = j - 1  # Index of line with closing }
            block_lines = lines[block_start:block_end+1]
            block_text = ''.join(block_lines)
            
            # If block doesn't have context, add ..Default::default()
            if 'context:' not in block_text:
                # Find the line before closing brace
                for k in range(block_end - 1, block_start, -1):
                    if lines[k].strip() and '}' not in lines[k]:
                        indent = len(lines[k]) - len(lines[k].lstrip())
                        # Insert ..Default::default() after this line
                        new_line = ' ' * indent + '..Default::default(),\n'
                        lines.insert(k + 1, new_line)
                        # Update indices
                        block_end += 1
                        i = block_end + 1
                        break
    
    result.append(lines[i])
    i += 1

with open('type_checker.rs', 'w') as f:
    f.writelines(result)

print('Done')
