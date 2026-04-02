with open('type_checker.rs', 'r') as f:
    lines = f.readlines()

# Find lines with TypeError { that need fixing
# Skip struct definition (line with pub struct before) and impl
indices_to_fix = []
for i, line in enumerate(lines):
    if 'TypeError {' in line:
        # Skip struct definition
        if i > 0 and 'pub struct' in lines[i-1]:
            continue
        # Skip impl block
        if i > 0 and 'impl TypeError' in lines[i-1]:
            continue
        # Check if this block already has context
        j = i + 1
        has_context = False
        brace_count = line.count('{') - line.count('}')
        while j < len(lines) and brace_count > 0:
            if 'context:' in lines[j]:
                has_context = True
                break
            brace_count += lines[j].count('{') - lines[j].count('}')
            j += 1
        if not has_context:
            indices_to_fix.append(i)

print(f"Found {len(indices_to_fix)} TypeError blocks to fix")

# Fix each block
for idx in reversed(indices_to_fix):  # Reverse to maintain line numbers
    # Find the closing line of this block
    j = idx + 1
    brace_count = lines[idx].count('{') - lines[idx].count('}')
    while j < len(lines) and brace_count > 0:
        brace_count += lines[j].count('{') - lines[j].count('}')
        j += 1
    
    # Insert context and help before the closing brace
    # Find the right indentation
    for k in range(j-1, idx, -1):
        if lines[k].strip() and '}' not in lines[k]:
            indent = len(lines[k]) - len(lines[k].lstrip())
            lines.insert(j-1, ' ' * indent + 'context: None,\n')
            lines.insert(j, ' ' * indent + 'help: None,\n')
            break

with open('type_checker.rs', 'w') as f:
    f.writelines(lines)

print('Done')
