#!/usr/bin/env python3
"""
Migration Helper Script for actix-web to axum and PostgreSQL to Turso

This script helps identify and prepare files for migration.
It doesn't fully automate the process but provides useful analysis.
"""

import os
import re
from pathlib import Path
from collections import defaultdict

class MigrationAnalyzer:
    def __init__(self, rust_backend_path):
        self.root = Path(rust_backend_path)
        self.stats = defaultdict(int)
        self.files_to_migrate = []
        
    def analyze_file(self, file_path):
        """Analyze a single Rust file for migration requirements"""
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()
            
        issues = []
        
        # Check for actix-web dependencies
        if 'actix_web' in content or 'actix-web' in content:
            issues.append('actix-web')
            self.stats['actix_web_files'] += 1
            
        # Check for sqlx dependencies
        if 'sqlx::' in content:
            issues.append('sqlx')
            self.stats['sqlx_files'] += 1
            # Count queries
            self.stats['sqlx_queries'] += len(re.findall(r'sqlx::query', content))
            
        # Check for PostgreSQL-specific syntax
        if '$1' in content or 'JSONB' in content:
            issues.append('postgres_syntax')
            self.stats['postgres_files'] += 1
            
        # Check for actix-ws
        if 'actix_ws' in content:
            issues.append('actix_ws')
            self.stats['websocket_files'] += 1
            
        # Check for actix-files
        if 'actix_files' in content:
            issues.append('actix_files')
            self.stats['static_files'] += 1
            
        if issues:
            return {
                'path': str(file_path.relative_to(self.root)),
                'issues': issues
            }
        return None
    
    def analyze_directory(self, directory):
        """Recursively analyze all Rust files in directory"""
        for rs_file in Path(directory).rglob('*.rs'):
            result = self.analyze_file(rs_file)
            if result:
                self.files_to_migrate.append(result)
    
    def generate_report(self):
        """Generate migration analysis report"""
        print("=" * 80)
        print("MIGRATION ANALYSIS REPORT")
        print("=" * 80)
        print()
        
        print("Overall Statistics:")
        print(f"  Files using actix-web: {self.stats['actix_web_files']}")
        print(f"  Files using sqlx: {self.stats['sqlx_files']}")
        print(f"  Total sqlx queries: {self.stats['sqlx_queries']}")
        print(f"  Files with PostgreSQL syntax: {self.stats['postgres_files']}")
        print(f"  Files with WebSocket: {self.stats['websocket_files']}")
        print(f"  Files with static file serving: {self.stats['static_files']}")
        print()
        
        # Group files by category
        by_category = defaultdict(list)
        for file_info in self.files_to_migrate:
            path = file_info['path']
            if path.startswith('src/services/'):
                by_category['services'].append(file_info)
            elif path.startswith('src/routes/'):
                by_category['routes'].append(file_info)
            elif path.startswith('src/middleware/'):
                by_category['middleware'].append(file_info)
            else:
                by_category['other'].append(file_info)
        
        for category, files in sorted(by_category.items()):
            print(f"\n{category.upper()} ({len(files)} files):")
            print("-" * 80)
            for file_info in sorted(files, key=lambda x: x['path']):
                issues_str = ', '.join(file_info['issues'])
                print(f"  {file_info['path']}")
                print(f"    Issues: {issues_str}")
        
        print()
        print("=" * 80)
        print("PRIORITY RECOMMENDATIONS")
        print("=" * 80)
        print("""
1. HIGH PRIORITY - Core Infrastructure:
   - src/db.rs (✅ DONE)
   - src/error.rs
   - src/middleware/auth.rs
   - src/services/user.rs
   - src/services/auth.rs
   - src/services/config.rs

2. MEDIUM PRIORITY - Main Routes:
   - src/routes/auth.rs
   - src/routes/users.rs
   - src/routes/chats.rs
   - src/routes/openai.rs
   - src/main.rs

3. LOW PRIORITY - Additional Features:
   - All other service and route files
   - WebSocket handlers
   - Socket.IO implementation

Suggested approach:
1. Complete HIGH priority files first
2. Test basic functionality (auth, user management, config)
3. Then migrate MEDIUM priority (main API routes)
4. Finally handle LOW priority (additional features)
""")

def main():
    script_dir = Path(__file__).parent
    rust_backend = script_dir
    
    print(f"Analyzing: {rust_backend}")
    print()
    
    analyzer = MigrationAnalyzer(rust_backend)
    
    # Analyze source files
    analyzer.analyze_directory(rust_backend / 'src')
    
    # Generate report
    analyzer.generate_report()
    
    # Save file list
    output_file = rust_backend / 'MIGRATION_FILES.txt'
    with open(output_file, 'w') as f:
        f.write("Files Requiring Migration\n")
        f.write("=" * 80 + "\n\n")
        for file_info in sorted(analyzer.files_to_migrate, key=lambda x: x['path']):
            f.write(f"{file_info['path']}\n")
            f.write(f"  Issues: {', '.join(file_info['issues'])}\n\n")
    
    print(f"\nDetailed file list saved to: {output_file}")

if __name__ == '__main__':
    main()
