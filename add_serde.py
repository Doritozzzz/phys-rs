import os
import re

def process_file(filepath):
    with open(filepath, 'r', encoding='utf-8') as f:
        content = f.read()

    # Find #[derive(..., Component, ...)] and add serde::Serialize, serde::Deserialize
    def replacer(match):
        inner = match.group(1)
        if 'Serialize' not in inner:
            return f"#[derive({inner}, serde::Serialize, serde::Deserialize)]"
        return match.group(0)

    new_content = re.sub(r'#\[derive\((.*?)\)\]', replacer, content)

    if new_content != content:
        with open(filepath, 'w', encoding='utf-8') as f:
            f.write(new_content)
        print(f"Updated {filepath}")

for root, _, files in os.walk('src/components'):
    for file in files:
        if file.endswith('.rs'):
            process_file(os.path.join(root, file))

process_file('src/core/coordinates.rs')
