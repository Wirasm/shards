# Prime: Load Project Context

## Objective
Build comprehensive understanding of the codebase by analyzing structure, documentation, and key files.

## Process

### 1. Analyze Project Structure
If this is a git repository, list tracked files:
```bash
git ls-files
```

Show directory structure:
```bash
tree -L 3 -I 'node_modules|__pycache__|.git|dist|build'
```
(or use `ls -la` and explore key directories if tree is not available)

### 2. Read Core Documentation
- Read README files at project root and major directories
- Read any architecture documentation
- Review steering documents for project context (already loaded in context)
- Avoid reading anything in examples or content_plan folders

### 3. Identify Key Files
Based on the structure, identify and read:
- Main entry points (main.py, index.ts, app.py, etc.)
- Core configuration files (pyproject.toml, package.json, tsconfig.json)
- Key model/schema definitions
- Important service or controller files

### 4. Understand Current State (if git repository)
Check recent activity:
```bash
git log -10 --oneline
```

Check current branch and status:
```bash
git status
```

## Output Report
Provide a concise summary covering:

### Project Overview
- Purpose and type of application
- Primary technologies and frameworks
- Current version/state

### Architecture
- Overall structure and organization
- Key architectural patterns identified
- Important directories and their purposes

### Tech Stack
- Languages and versions
- Frameworks and major libraries
- Build tools and package managers
- Testing frameworks

### Core Principles
- Code style and conventions observed
- Documentation standards
- Testing approach

### Current State
- Active branch (if git repository)
- Recent changes or development focus (if git repository)
- Any immediate observations or concerns

**Make this summary easy to scan - use bullet points and clear headers.**
