with open('type_checker.rs', 'r') as f:
    lines = f.readlines()

# Find impl block range
impl_start = None
impl_end = None
brace_count = 0
in_impl = False

for i, line in enumerate(lines):
    if 'impl TypeError' in line:
        impl_start = i
        in_impl = True
    if in_impl:
        brace_count += line.count('{') - line.count('}')
        if brace_count == 0 and '{' in lines[impl_start:i+1].__str__():
            impl_end = i
            break

print(f"impl block: {impl_start} to {impl_end}")

# Find lines with TypeError { that need fixing
indices_to_fix = []
for i, line in enumerate(lines):
    if 'TypeError {' in line:
        # Skip struct definition
        if i > 0 and 'pub struct' in lines[i-1]:
            continue
        # Skip impl block
        if impl_start and impl_end and impl_start <= i <= impl_end:
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
for idx in reversed(indices_to_fix):
    j = idx + 1
    brace_count = lines[idx].count('{') - lines[idx].count('}')
    while j < len(lines) and brace_count > 0:
        brace_count += lines[j].count('{') - lines[j].count('}')
        j += 1
    
    for k in range(j-1, idx, -1):
        if lines[k].strip() and '}' not in lines[k]:
            indent = len(lines[k]) - len(lines[k].lstrip())
            lines.insert(j-1, ' ' * indent + 'context: None,\n')
            lines.insert(j, ' ' * indent + 'help: None,\n')
            break

with open('type_checker.rs', 'w') as f:
    f.writelines(lines)

print('Done')
