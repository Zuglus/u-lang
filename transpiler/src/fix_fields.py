import re

with open('type_checker.rs', 'r') as f:
    content = f.read()

# Pattern to match TypeError { ... } blocks without context/help
# This is complex - let's do it line by line approach instead

lines = content.split('\n')
result = []
in_type_error = False
type_error_start = -1
type_error_lines = []

for i, line in enumerate(lines):
    if 'TypeError {' in line and 'context' not in line:
        in_type_error = True
        type_error_start = i
        type_error_lines = [line]
    elif in_type_error:
        type_error_lines.append(line)
        if line.strip() == '}' or line.strip() == '}),':
            # End of TypeError block - add context and help before closing
            # Find the line with span and add after it
            new_lines = []
            for j, tel in enumerate(type_error_lines):
                new_lines.append(tel)
                if 'span:' in tel and 'context' not in type_error_lines[j+1] if j+1 < len(type_error_lines) else True:
                    indent = len(tel) - len(tel.lstrip())
                    new_lines.append(' ' * indent + 'context: None,')
                    new_lines.append(' ' * indent + 'help: None,')
            result.extend(new_lines)
            in_type_error = False
            type_error_lines = []
        elif i == len(lines) - 1:
            # End of file
            result.extend(type_error_lines)
    else:
        result.append(line)

with open('type_checker.rs', 'w') as f:
    f.write('\n'.join(result))

print('Done')
