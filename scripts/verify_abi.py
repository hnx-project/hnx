#!/usr/bin/env python3
"""
ABI Verification Script

This script verifies that the kernel's internal ABI definitions are consistent
with the public C header files.
"""

import re
import sys
from pathlib import Path

def extract_constants_from_rust(filepath):
    """Extract constants from Rust ABI file"""
    constants = {}
    with open(filepath, 'r') as f:
        content = f.read()
    
    # Match Rust const definitions
    pattern = r'pub const (\w+):\s*\w+\s*=\s*(0x[0-9a-fA-F]+|\d+);'
    for match in re.finditer(pattern, content):
        name = match.group(1)
        value = match.group(2)
        # Convert to decimal if hex
        if value.startswith('0x'):
            constants[name] = str(int(value, 16))
        else:
            constants[name] = value
    
    return constants

def extract_constants_from_c(filepath):
    """Extract constants from C header files"""
    constants = {}
    
    # Process all C header files in the ABI directory
    if filepath.is_dir():
        for header_file in filepath.glob("*.h"):
            with open(header_file, 'r') as f:
                content = f.read()
            
            # Match C #define statements
            pattern = r'#define\s+(\w+)\s+(0x[0-9a-fA-F]+|\d+)'
            for match in re.finditer(pattern, content):
                name = match.group(1)
                value = match.group(2)
                # Convert to decimal if hex
                if value.startswith('0x'):
                    constants[name] = str(int(value, 16))
                else:
                    constants[name] = value
    else:
        with open(filepath, 'r') as f:
            content = f.read()
        
        # Match C #define statements
        pattern = r'#define\s+(\w+)\s+(0x[0-9a-fA-F]+|\d+)'
        for match in re.finditer(pattern, content):
            name = match.group(1)
            value = match.group(2)
            # Convert to decimal if hex
            if value.startswith('0x'):
                constants[name] = str(int(value, 16))
            else:
                constants[name] = value
    
    return constants

def main():
    # Define paths
    kernel_abi = Path("src/kernel/src/abi/mod.rs")
    include_abi = Path("include/hnx/abi")
    
    # Extract constants
    kernel_consts = extract_constants_from_rust(kernel_abi)
    include_consts = extract_constants_from_c(include_abi)
    
    # Check consistency
    errors = []
    all_names = set(kernel_consts.keys()) | set(include_consts.keys())
    
    for name in all_names:
        if name in kernel_consts and name in include_consts:
            if kernel_consts[name] != include_consts[name]:
                errors.append(f"{name}: kernel={kernel_consts[name]}, include={include_consts[name]}")
        elif name in kernel_consts:
            # Check if this is a special constant that doesn't need to be in headers
            if not name.startswith(('ZX_', 'HNX_ABI_')):
                errors.append(f"{name}: defined in kernel but not in include headers")
        else:
            errors.append(f"{name}: defined in include headers but not in kernel")
    
    if errors:
        print("ABI inconsistency found:", file=sys.stderr)
        for error in errors:
            print(f"  {error}", file=sys.stderr)
        sys.exit(1)
    
    print("ABI headers are consistent")
    return 0

if __name__ == "__main__":
    sys.exit(main())