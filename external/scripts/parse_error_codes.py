#!/usr/bin/env python3
"""
Parse Stata error codes from markdown and generate structured formats.

Input: external/stata-docs/markdown/error-codes-raw.md
Outputs:
  - external/stata-docs/markdown/error-codes.md (cleaned, structured)
  - external/stata-docs/structured/error-codes.toml
  - external/stata-docs/structured/error-codes.json
  - external/stata-docs/structured/error-codes.csv
  - src/error/official_codes.rs (generated Rust code)

Usage:
    python external/scripts/parse_error_codes.py
"""

import re
import json
import csv
from dataclasses import dataclass, asdict
from typing import List, Optional
from pathlib import Path
from datetime import date


@dataclass
class StataErrorCode:
    """Represents a single Stata error code."""
    code: int
    name: str
    description: str
    category: Optional[str] = None
    references: Optional[List[str]] = None  # Cross-references to other manual sections
    stata_version: str = "18"

    def __post_init__(self):
        # Clean up description (normalize excessive whitespace)
        self.description = re.sub(r'\s+', ' ', self.description).strip()

        # Clean up name (remove newlines, normalize whitespace)
        self.name = re.sub(r'\s+', ' ', self.name).strip()

        # Assign category based on code range
        if self.category is None:
            self.category = self._assign_category()

    def _assign_category(self) -> str:
        """Assign category based on Stata's error code grouping."""
        if 1 <= self.code <= 99:
            return "General"
        elif 100 <= self.code <= 199:
            return "Syntax/Command"
        elif 300 <= self.code <= 399:
            return "Previously stored result"
        elif 400 <= self.code <= 499:
            return "Statistical problems"
        elif 500 <= self.code <= 599:
            return "Matrix manipulation"
        elif 600 <= self.code <= 699:
            return "File I/O"
        elif 700 <= self.code <= 799:
            return "Operating system"
        elif 900 <= self.code <= 999:
            return "Memory/Resources"
        elif 1000 <= self.code <= 1999:
            return "System limits"
        elif 2000 <= self.code <= 2999:
            return "Non-errors (continuation)"
        elif 3000 <= self.code <= 3999:
            return "Mata runtime"
        elif 4000 <= self.code <= 4999:
            return "Class system"
        elif 7100 <= self.code <= 7199:
            return "Python runtime"
        elif 9000 <= self.code <= 9999:
            return "System failure"
        else:
            return "Other"


def parse_error_codes_markdown(md_path: Path) -> List[StataErrorCode]:
    """
    Parse error-codes.md into structured error code objects.

    Expected format (flexible):
    ## r(1) - Generic error
    Catchall error code when no specific error applies.

    OR:

    r(1) Generic error
    Catchall error code...

    OR table format:

    | Code | Name | Description |
    |------|------|-------------|
    | 1    | Generic error | Catchall... |
    """
    with open(md_path) as f:
        content = f.read()

    errors = []

    # Try parsing as structured markdown (## r(code) format)
    header_pattern = r'^##\s+r\((\d+)\)\s*[-–—]?\s*(.+?)$'
    matches = re.finditer(header_pattern, content, re.MULTILINE)

    sections = []
    for match in matches:
        code = int(match.group(1))
        name = match.group(2).strip()
        start_pos = match.end()
        sections.append((code, name, start_pos))

    if sections:
        # Parse header-based format
        for i, (code, name, start_pos) in enumerate(sections):
            # Find end of this section (start of next section or EOF)
            if i + 1 < len(sections):
                end_pos = sections[i + 1][2]
            else:
                end_pos = len(content)

            description = content[start_pos:end_pos].strip()

            # Remove separator lines
            description = re.sub(r'^---+$', '', description, flags=re.MULTILINE)
            description = ' '.join(description.split())

            errors.append(StataErrorCode(
                code=code,
                name=name,
                description=description if description else "No description available"
            ))

    else:
        # Try parsing as table format
        table_pattern = r'\|\s*(\d+)\s*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|'
        for match in re.finditer(table_pattern, content):
            code = int(match.group(1))
            name = match.group(2).strip()
            description = match.group(3).strip()

            errors.append(StataErrorCode(
                code=code,
                name=name,
                description=description
            ))

    if not errors:
        # Try numbered list format (e.g., " 1. description here\n 2. another desc")
        # This is common in Stata Programming Manual appendix
        numbered_list_pattern = r'^\s*(\d+)\.\s+(.+?)$'
        matches = list(re.finditer(numbered_list_pattern, content, re.MULTILINE))

        if matches:
            for i, match in enumerate(matches):
                code = int(match.group(1))
                description_first_line = match.group(2).strip()

                # Collect continuation lines (lines that don't start with a number)
                start_pos = match.end()
                if i + 1 < len(matches):
                    end_pos = matches[i + 1].start()
                else:
                    end_pos = len(content)

                continuation = content[start_pos:end_pos].strip()

                # Combine first line and continuation
                full_description = description_first_line
                if continuation and not re.match(r'^\s*\d+\.', continuation):
                    full_description += ' ' + continuation

                # Extract name (first line only, or first sentence if short)
                # Clean up excessive whitespace in name extraction
                first_line_clean = re.sub(r'\s+', ' ', description_first_line)

                # Try to get just the error message template (before explanation)
                # Look for pattern: "message\n    Explanation" or "message. Explanation"
                name_parts = first_line_clean.split('\n', 1)
                name_candidate = name_parts[0].strip()

                # If name is too long, try to get first sentence
                if len(name_candidate) > 100:
                    sentence_match = re.match(r'^([^.]+\.)', name_candidate)
                    name = sentence_match.group(1).strip() if sentence_match else name_candidate[:80]
                else:
                    name = name_candidate

                # Extract cross-references like [D] append, [U] 12.5, [R] ranksum
                # Pattern captures: [MANUAL] section.subsection.subsubsection
                ref_pattern = r'\[([A-Z]+)\]\s+([\d\.]+(?:\s+[a-zA-Z][\w\s-]*)?|[a-zA-Z][\w\s-]+)'
                references = re.findall(ref_pattern, full_description)

                # Clean up and deduplicate references
                ref_list = []
                seen = set()
                for manual, section in references:
                    section_clean = section.strip()
                    # Normalize whitespace (including newlines)
                    section_clean = re.sub(r'\s+', ' ', section_clean)
                    # Fix hyphenation artifacts (e.g., "num- list" -> "numlist")
                    section_clean = re.sub(r'(\w+)-\s+(\w+)', r'\1\2', section_clean)
                    # Remove trailing punctuation
                    section_clean = section_clean.rstrip('.,;:')

                    ref_key = f"[{manual}] {section_clean}"
                    if ref_key not in seen:
                        ref_list.append(ref_key)
                        seen.add(ref_key)

                errors.append(StataErrorCode(
                    code=code,
                    name=name,
                    description=full_description,
                    references=ref_list if ref_list else None
                ))

    if not errors:
        # Try simple r(code) pattern without headers
        simple_pattern = r'r\((\d+)\)\s*[:-]?\s*([^\n]+)'
        for match in re.finditer(simple_pattern, content):
            code = int(match.group(1))
            rest = match.group(2).strip()

            # Try to split name and description
            if '.' in rest:
                parts = rest.split('.', 1)
                name = parts[0].strip()
                description = parts[1].strip() if len(parts) > 1 else ""
            elif '-' in rest:
                parts = rest.split('-', 1)
                name = parts[0].strip()
                description = parts[1].strip() if len(parts) > 1 else ""
            else:
                name = rest
                description = ""

            errors.append(StataErrorCode(
                code=code,
                name=name,
                description=description if description else name
            ))

    # Sort by code
    errors.sort(key=lambda e: e.code)

    # Remove duplicates (keep first occurrence)
    seen_codes = set()
    unique_errors = []
    for err in errors:
        if err.code not in seen_codes:
            unique_errors.append(err)
            seen_codes.add(err.code)

    return unique_errors


def export_to_markdown(errors: List[StataErrorCode], output_path: Path):
    """Export to clean, structured markdown."""
    with open(output_path, 'w') as f:
        f.write('# Stata Error Codes Reference\n\n')
        f.write('Extracted from: Programming Manual (p.pdf), pages 209-223\n')
        f.write('Stata Version: 18\n')
        f.write(f'Extraction Date: {date.today()}\n')
        f.write(f'Total Error Codes: {len(errors)}\n\n')
        f.write('---\n\n')

        for err in errors:
            f.write(f'## r({err.code}) - {err.name}\n\n')
            if err.category:
                f.write(f'**Category**: {err.category}\n\n')
            f.write(f'{err.description}\n\n')
            if err.references:
                f.write(f'**See also**: {", ".join(err.references)}\n\n')
            f.write('---\n\n')


def export_to_toml(errors: List[StataErrorCode], output_path: Path):
    """Export to TOML for Rust inclusion."""
    with open(output_path, 'w') as f:
        f.write('# Stata Error Codes - Extracted from p.pdf\n')
        f.write('# Source: Programming Manual, Appendix (pages 209-223)\n')
        f.write(f'# Stata Version: 18\n')
        f.write(f'# Extraction Date: {date.today()}\n')
        f.write(f'# Total Codes: {len(errors)}\n\n')

        for err in errors:
            f.write('[[error]]\n')
            f.write(f'code = {err.code}\n')
            f.write(f'name = "{err.name}"\n')
            if err.category:
                f.write(f'category = "{err.category}"\n')

            # Handle multiline descriptions
            desc = err.description.replace('"', '\\"')
            if '\n' in desc or len(desc) > 80:
                f.write(f'description = """\\\n{desc}\\\n"""\n')
            else:
                f.write(f'description = "{desc}"\n')

            f.write('\n')


def export_to_json(errors: List[StataErrorCode], output_path: Path):
    """Export to JSON for machine consumption."""
    # Filter out None values from each error dict
    errors_clean = []
    for err in errors:
        err_dict = asdict(err)
        # Remove None/null fields
        err_dict = {k: v for k, v in err_dict.items() if v is not None}
        errors_clean.append(err_dict)

    data = {
        "source": "Stata Programming Manual v18",
        "pages": "209-223",
        "extraction_date": str(date.today()),
        "stata_version": "18",
        "total_codes": len(errors),
        "errors": errors_clean
    }

    with open(output_path, 'w') as f:
        json.dump(data, f, indent=2)


def export_to_csv(errors: List[StataErrorCode], output_path: Path):
    """Export to CSV for spreadsheet analysis."""
    with open(output_path, 'w', newline='') as f:
        writer = csv.writer(f)
        writer.writerow(['Code', 'Name', 'Category', 'Description'])

        for err in errors:
            writer.writerow([
                err.code,
                err.name,
                err.category or '',
                err.description
            ])


def generate_rust_code(errors: List[StataErrorCode], output_path: Path):
    """Generate Rust source code with error code definitions."""
    with open(output_path, 'w') as f:
        f.write('// DO NOT EDIT - Generated from external/stata-docs/error-codes.toml\n')
        f.write('// Source: Stata Programming Manual v18, pages 209-223\n')
        f.write(f'// Generation Date: {date.today()}\n')
        f.write(f'// Total Codes: {len(errors)}\n\n')

        f.write('/// Official Stata error code from documentation\n')
        f.write('#[derive(Debug, Clone)]\n')
        f.write('pub struct OfficialErrorCode {\n')
        f.write('    pub code: u32,\n')
        f.write('    pub name: &\'static str,\n')
        f.write('    pub category: &\'static str,\n')
        f.write('    pub description: &\'static str,\n')
        f.write('}\n\n')

        f.write('/// All official Stata error codes from Programming Manual\n')
        f.write('pub const OFFICIAL_ERROR_CODES: &[OfficialErrorCode] = &[\n')

        for err in errors:
            # Escape strings for Rust
            name = err.name.replace('\\', '\\\\').replace('"', '\\"')
            desc = err.description.replace('\\', '\\\\').replace('"', '\\"')
            category = err.category if err.category else "General"

            f.write(f'    OfficialErrorCode {{\n')
            f.write(f'        code: {err.code},\n')
            f.write(f'        name: "{name}",\n')
            f.write(f'        category: "{category}",\n')
            f.write(f'        description: "{desc}",\n')
            f.write(f'    }},\n')

        f.write('];\n\n')

        f.write('/// Look up official error code by number\n')
        f.write('pub fn lookup_official_error(code: u32) -> Option<&\'static OfficialErrorCode> {\n')
        f.write('    OFFICIAL_ERROR_CODES.iter().find(|e| e.code == code)\n')
        f.write('}\n\n')

        f.write('/// Get all error codes as a sorted list\n')
        f.write('pub fn all_error_codes() -> Vec<u32> {\n')
        f.write('    OFFICIAL_ERROR_CODES.iter().map(|e| e.code).collect()\n')
        f.write('}\n')


def main():
    """Main extraction pipeline."""
    project_root = Path(__file__).parent.parent.parent
    external_dir = project_root / 'external'

    # Input
    raw_md = external_dir / 'stata-docs' / 'markdown' / 'error-codes-raw.md'

    # Outputs
    clean_md = external_dir / 'stata-docs' / 'markdown' / 'error-codes.md'
    toml_out = external_dir / 'stata-docs' / 'structured' / 'error-codes.toml'
    json_out = external_dir / 'stata-docs' / 'structured' / 'error-codes.json'
    csv_out = external_dir / 'stata-docs' / 'structured' / 'error-codes.csv'
    rust_out = project_root / 'src' / 'error' / 'official_codes.rs'

    # Check input exists
    if not raw_md.exists():
        print(f'❌ Input file not found: {raw_md}')
        print('\nPlease convert p.pdf pages 209-223 to markdown first:')
        print(f'  1. Extract pages 209-223 from p.pdf')
        print(f'  2. Convert to markdown')
        print(f'  3. Save as: {raw_md}')
        return 1

    print(f'📖 Parsing error codes from: {raw_md}')

    # Parse
    errors = parse_error_codes_markdown(raw_md)

    if not errors:
        print('❌ No error codes found in input file')
        print('Check the format of the markdown file')
        return 1

    print(f'✅ Extracted {len(errors)} error codes')
    print(f'   Code range: {min(e.code for e in errors)} - {max(e.code for e in errors)}')

    # Export to all formats
    print('\n📝 Generating outputs:')

    export_to_markdown(errors, clean_md)
    print(f'  ✅ Markdown: {clean_md}')

    export_to_toml(errors, toml_out)
    print(f'  ✅ TOML: {toml_out}')

    export_to_json(errors, json_out)
    print(f'  ✅ JSON: {json_out}')

    export_to_csv(errors, csv_out)
    print(f'  ✅ CSV: {csv_out}')

    generate_rust_code(errors, rust_out)
    print(f'  ✅ Rust: {rust_out}')

    print('\n🎉 Extraction complete!')
    print(f'\nNext steps:')
    print(f'  1. Review generated files for accuracy')
    print(f'  2. Run validation: python external/scripts/validate_extraction.py')
    print(f'  3. Build project: cargo build')

    return 0


if __name__ == '__main__':
    exit(main())
