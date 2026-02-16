#!/usr/bin/env python3
"""
Validate extracted Stata error codes for completeness and correctness.

Checks:
  1. Required error codes are present
  2. No duplicate codes
  3. All codes have descriptions
  4. Code ranges are reasonable
  5. Output files are consistent

Usage:
    python external/scripts/validate_extraction.py
"""

import json
import csv
from pathlib import Path
from typing import List, Dict, Set


# Error codes we MUST have (commonly seen in practice)
REQUIRED_CODES = [
    1,    # Generic error
    198,  # Invalid syntax
    199,  # Unrecognized command
    601,  # File not found
    603,  # File not readable
    950,  # Memory error
]

# Known error code categories (for validation)
KNOWN_CATEGORIES = {
    'General', 'Syntax', 'File', 'Memory', 'Network',
    'Data', 'Variable', 'Matrix', 'Macro', 'Programming'
}


def load_json(json_path: Path) -> Dict:
    """Load and return JSON error codes."""
    with open(json_path) as f:
        return json.load(f)


def load_csv(csv_path: Path) -> List[Dict]:
    """Load and return CSV error codes."""
    with open(csv_path) as f:
        reader = csv.DictReader(f)
        return list(reader)


def check_required_codes(codes: List[int]) -> List[str]:
    """Check that all required error codes are present."""
    errors = []
    missing = [c for c in REQUIRED_CODES if c not in codes]

    if missing:
        errors.append(f'❌ Missing required error codes: {missing}')
        errors.append('   These are commonly seen errors that MUST be documented')

    return errors


def check_duplicates(codes: List[int]) -> List[str]:
    """Check for duplicate error codes."""
    errors = []
    seen = set()
    duplicates = set()

    for code in codes:
        if code in seen:
            duplicates.add(code)
        seen.add(code)

    if duplicates:
        errors.append(f'❌ Duplicate error codes found: {sorted(duplicates)}')

    return errors


def check_descriptions(error_data: List[Dict]) -> List[str]:
    """Check that all errors have descriptions."""
    errors = []
    missing = []

    for err in error_data:
        code = err.get('code') or err.get('Code')
        desc = err.get('description') or err.get('Description')

        if not desc or desc.strip() == '':
            missing.append(code)

    if missing:
        errors.append(f'❌ Error codes missing descriptions: {missing[:10]}')
        if len(missing) > 10:
            errors.append(f'   ... and {len(missing) - 10} more')

    return errors


def check_code_ranges(codes: List[int]) -> List[str]:
    """Check for suspicious gaps in error code ranges."""
    warnings = []
    sorted_codes = sorted(codes)

    gaps = []
    for i in range(len(sorted_codes) - 1):
        gap = sorted_codes[i + 1] - sorted_codes[i]
        if gap > 20:  # Large gap might indicate missing codes
            gaps.append((sorted_codes[i], sorted_codes[i + 1], gap))

    if gaps:
        warnings.append('⚠️  Large gaps in error codes detected:')
        for start, end, gap in gaps[:5]:
            warnings.append(f'   Gap of {gap} between r({start}) and r({end})')
        if len(gaps) > 5:
            warnings.append(f'   ... and {len(gaps) - 5} more gaps')
        warnings.append('   (This might be normal, but verify against PDF)')

    return warnings


def check_file_consistency(json_path: Path, csv_path: Path) -> List[str]:
    """Check that JSON and CSV have the same error codes."""
    errors = []

    json_data = load_json(json_path)
    csv_data = load_csv(csv_path)

    json_codes = {e['code'] for e in json_data['errors']}
    csv_codes = {int(e['Code']) for e in csv_data}

    if json_codes != csv_codes:
        only_json = json_codes - csv_codes
        only_csv = csv_codes - json_codes

        if only_json:
            errors.append(f'❌ Codes in JSON but not CSV: {sorted(only_json)[:10]}')
        if only_csv:
            errors.append(f'❌ Codes in CSV but not JSON: {sorted(only_csv)[:10]}')

    return errors


def check_format_validity(json_path: Path, csv_path: Path) -> List[str]:
    """Check that files are valid JSON/CSV."""
    errors = []

    try:
        load_json(json_path)
    except json.JSONDecodeError as e:
        errors.append(f'❌ Invalid JSON: {e}')

    try:
        load_csv(csv_path)
    except Exception as e:
        errors.append(f'❌ Invalid CSV: {e}')

    return errors


def print_summary(json_data: Dict):
    """Print summary statistics."""
    errors = json_data['errors']
    codes = [e['code'] for e in errors]

    print('\n📊 Summary Statistics:')
    print(f'  Total error codes: {len(codes)}')
    print(f'  Code range: {min(codes)} - {max(codes)}')
    print(f'  Source: {json_data["source"]}')
    print(f'  Pages: {json_data["pages"]}')
    print(f'  Extraction date: {json_data["extraction_date"]}')

    # Count codes by range
    ranges = [
        (1, 99, 'General'),
        (100, 199, 'Syntax/Command'),
        (200, 299, 'Data/Variable'),
        (300, 599, 'File/IO'),
        (600, 699, 'File errors'),
        (700, 899, 'Network/System'),
        (900, 999, 'Memory/Resources'),
    ]

    print('\n  Error codes by range:')
    for start, end, label in ranges:
        count = sum(1 for c in codes if start <= c <= end)
        if count > 0:
            print(f'    r({start:3d})-r({end:3d}) [{label:20s}]: {count:3d} codes')


def main():
    """Run all validation checks."""
    project_root = Path(__file__).parent.parent.parent
    external_dir = project_root / 'external'

    json_path = external_dir / 'stata-docs' / 'structured' / 'error-codes.json'
    csv_path = external_dir / 'stata-docs' / 'structured' / 'error-codes.csv'

    # Check files exist
    if not json_path.exists():
        print(f'❌ JSON file not found: {json_path}')
        print('\nRun parse_error_codes.py first to generate structured data')
        return 1

    if not csv_path.exists():
        print(f'❌ CSV file not found: {csv_path}')
        print('\nRun parse_error_codes.py first to generate structured data')
        return 1

    print('🔍 Validating extracted error codes...\n')

    # Check format validity first
    format_errors = check_format_validity(json_path, csv_path)
    if format_errors:
        for err in format_errors:
            print(err)
        return 1

    # Load data
    json_data = load_json(json_path)
    csv_data = load_csv(csv_path)

    errors = json_data['errors']
    codes = [e['code'] for e in errors]

    # Run all checks
    all_errors = []
    all_warnings = []

    all_errors.extend(check_required_codes(codes))
    all_errors.extend(check_duplicates(codes))
    all_errors.extend(check_descriptions(errors))
    all_errors.extend(check_file_consistency(json_path, csv_path))

    all_warnings.extend(check_code_ranges(codes))

    # Print results
    if all_errors:
        print('❌ VALIDATION FAILED\n')
        for err in all_errors:
            print(err)
        print()

    if all_warnings:
        for warn in all_warnings:
            print(warn)
        print()

    if not all_errors and not all_warnings:
        print('✅ All validation checks passed!')

    # Print summary
    print_summary(json_data)

    # Final status
    if all_errors:
        print('\n❌ Fix errors before proceeding')
        return 1
    else:
        print('\n✅ Validation complete - extraction is valid')

        if all_warnings:
            print('⚠️  Review warnings above')

        print('\nNext steps:')
        print('  1. Review generated markdown: external/stata-docs/markdown/error-codes.md')
        print('  2. Verify against original PDF (pages 209-223)')
        print('  3. Build Rust project: cargo build')

        return 0


if __name__ == '__main__':
    exit(main())
