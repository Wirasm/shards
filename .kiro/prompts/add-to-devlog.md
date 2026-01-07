---
description: "Add a structured daily entry to your development log with automatic technical progress tracking"
---

# Add to Development Log

## Daily Development Log Entry

### User Information Gathering

Ask the user these questions to build today's log entry:

**Core Questions:**
1. **What did you work on today?** (Features, bugs, refactoring, planning, etc.)
2. **How much time did you spend?** (Total hours, or breakdown by task if they prefer)
3. **What were the main accomplishments?** (What got completed or significant progress made)
4. **Any challenges or blockers encountered?** (Technical issues, decisions, learning curves)
5. **Key decisions made?** (Architecture choices, technology selections, approach changes)
6. **What's planned for next session?** (Next priorities or tasks)

**Optional Details:**
7. **Any new learnings or insights?** (Skills gained, patterns discovered, best practices learned)
8. **Kiro CLI usage highlights?** (Which prompts were most helpful, new workflows discovered)

### Date and Progress Tracking

First, determine the current date and calculate which day of the hackathon this is:

```bash
# Get current date
date +"%B %d, %Y (%A)"

# Calculate hackathon day (January 5, 2026 = Day 1)
# This will help number the daily entries correctly
```

### Technical Progress Analysis

After gathering user responses, automatically check technical progress using these commands:

#### Git Activity Analysis
```bash
# Check if we're in a git repository
if git rev-parse --git-dir > /dev/null 2>&1; then
    echo "Git repository detected"
    
    # Get today's commits (try different time formats)
    echo "=== Today's Commits ==="
    git log --since="$(date +%Y-%m-%d)" --oneline --author="$(git config user.name)" 2>/dev/null || echo "No commits found for today"
    
    # Get detailed commit info with line changes for today
    echo "=== Today's Detailed Changes ==="
    git log --since="$(date +%Y-%m-%d)" --stat --author="$(git config user.name)" 2>/dev/null || echo "No detailed changes for today"
    
    # Get total line changes for today
    echo "=== Today's Line Statistics ==="
    git log --since="$(date +%Y-%m-%d)" --author="$(git config user.name)" --pretty=tformat: --numstat 2>/dev/null | awk '{add+=$1; del+=$2} END {if(NR>0) printf "Lines added: %d, Lines removed: %d, Net change: %d\n", add, del, add-del; else print "No line changes today"}'
    
    # Check current branch and status
    echo "=== Repository Status ==="
    git status --porcelain
    echo "Current branch: $(git branch --show-current)"
    
    # Get total project statistics
    echo "=== Total Project Statistics ==="
    git log --author="$(git config user.name)" --pretty=tformat: --numstat 2>/dev/null | awk '{add+=$1; del+=$2} END {if(NR>0) printf "Total lines added: %d, Total lines removed: %d\n", add, del; else print "No statistics available"}'
    
    # Count total commits by user
    echo "=== Total Commits ==="
    git rev-list --count --author="$(git config user.name)" HEAD 2>/dev/null || echo "Cannot count commits"
    
else
    echo "Not in a git repository - skipping git analysis"
fi
```

#### GitHub Activity (if GitHub CLI is available)
```bash
# Check if gh CLI is available
which gh

# If available, get recent PR activity
gh pr list --author="@me" --state=all --limit=5

# Get recent issues activity
gh issue list --author="@me" --state=all --limit=5

# Check recent repository activity
gh repo view --json pushedAt,updatedAt
```

#### File Changes Analysis
```bash
# Show files changed today
git diff --name-only HEAD~1 HEAD 2>/dev/null || echo "No recent commits to compare"

# Show detailed line changes for today
git diff --stat HEAD~1 HEAD 2>/dev/null || echo "No recent commits to compare"

# Count different file types modified
find . -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.jsx" -o -name "*.tsx" -o -name "*.go" -o -name "*.java" -o -name "*.rb" -o -name "*.php" -o -name "*.cpp" -o -name "*.c" -o -name "*.cs" -o -name "*.html" -o -name "*.css" -o -name "*.md" | xargs ls -lt | head -10

# Get project file count and size
find . -type f \( -name "*.py" -o -name "*.js" -o -name "*.ts" -o -name "*.jsx" -o -name "*.tsx" -o -name "*.go" -o -name "*.java" -o -name "*.rb" -o -name "*.php" -o -name "*.cpp" -o -name "*.c" -o -name "*.cs" -o -name "*.html" -o -name "*.css" \) -not -path "./node_modules/*" -not -path "./.git/*" -not -path "./venv/*" -not -path "./__pycache__/*" | wc -l
```

### DEVLOG.md Entry Generation

Create a structured daily entry using this template and append it to `.kiro/devlog/devlog.md`:

```markdown
## Day [X] ([Full Date]) - [Main Focus/Theme] [[Total Time]h]

### 📊 **Daily Metrics**
- **Time Spent**: [Total hours] ([Breakdown if provided])
- **Commits Made**: [Number from git analysis]
- **Lines Added**: [From git stats]
- **Lines Removed**: [From git stats]
- **Net Lines**: [Added - Removed]
- **Files Modified**: [Count from git analysis]

### 🎯 **Accomplishments**
[User's main accomplishments - bullet points]

### 💻 **Technical Progress**
**Commits Made Today:**
[Git commit summary from commands above]

**Code Changes:**
[File changes summary with line counts]

**GitHub Activity:**
[PR/Issue activity if GitHub CLI available]

### 🔧 **Work Breakdown**
- **[Task Category]**: [Time] - [Description]
- **[Task Category]**: [Time] - [Description]
[Based on user's time breakdown]

### 🚧 **Challenges & Solutions**
[User's reported challenges and how they were addressed]

### 🧠 **Key Decisions**
[Important technical or architectural decisions made]

### 📚 **Learnings & Insights**
[New skills, patterns, or knowledge gained]

### ⚡ **Kiro CLI Usage**
[Prompts used, workflow improvements, custom commands created]

### 📋 **Next Session Plan**
[What's planned for the next development session]

---
```

### Metadata Update Instructions

After creating the daily entry, update the "Development Statistics" section at the top of the devlog:

**Required Updates:**
1. **Total Development Days**: Count existing "## Day" entries and add 1
2. **Total Hours Logged**: Parse previous entries for hours and add today's hours
3. **Total Commits**: Use the git commit count from analysis
4. **Lines of Code Added/Removed**: Use git statistics
5. **Files Modified**: Count from git status and changes

**Update Process:**
1. Read the current devlog file
2. Find the "### Overall Progress" section
3. Parse existing values (if any)
4. Add today's metrics to the totals
5. Replace the statistics section with updated values

**Example Update:**
```markdown
### Overall Progress
- **Total Development Days**: 1
- **Total Hours Logged**: 0.8h
- **Total Commits**: 1
- **Lines of Code Added**: 15268
- **Lines of Code Removed**: 0
- **Files Modified**: 85
```

### Implementation Instructions

1. **Calculate hackathon day and date**
   - Get current date using `date` command
   - Calculate which day of the hackathon this is (January 5, 2026 = Day 1)
   - Use this for the day number in the entry

2. **Check if devlog exists**
   - Look for `.kiro/devlog/devlog.md`
   - If it doesn't exist, create it using the template structure
   - If it exists, prepare to append the new entry

3. **Gather user information**
   - Ask questions in a conversational way
   - Allow users to skip optional questions
   - Be flexible with time tracking (total or breakdown)

4. **Execute technical analysis commands**
   - Run git commands to gather commit and file change data
   - Calculate detailed line statistics for today and total project
   - Try GitHub CLI commands (gracefully handle if not available)
   - Parse and summarize the technical data with specific metrics

5. **Generate and append entry**
   - Use the template above with user responses and technical data
   - Include detailed daily metrics section with code statistics
   - Use calculated day number and current date
   - Append to `.kiro/devlog/devlog.md` file

6. **Update summary statistics**
   - Parse the git output to extract key metrics
   - Update the "Development Statistics" section at the top of the devlog
   - Calculate and update:
     - Total Development Days (count existing day entries + 1)
     - Total Hours Logged (sum from previous entries + today's hours)
     - Total Commits (from git analysis)
     - Lines of Code Added/Removed (from git analysis)
     - Files Modified count
   - Use regex/text parsing to update the existing statistics section

7. **Provide summary**
   - Confirm the entry was added to `.kiro/devlog/devlog.md`
   - Show a brief summary of metrics captured
   - Suggest next steps if appropriate

### Error Handling

- **Git not available**: Skip git analysis, note in entry
- **GitHub CLI not available**: Skip GitHub analysis, note limitation
- **No commits today**: Note "No commits made today" in technical progress
- **DEVLOG.md locked/permissions**: Provide error message and manual instructions

### Quality Guidelines

- Keep entries scannable with clear headers and bullet points
- Focus on factual progress and concrete accomplishments
- Include both high-level summary and technical details
- Maintain consistent formatting across entries
- Balance brevity with useful detail

### Success Criteria

- User can quickly log daily progress without friction
- Technical progress is automatically captured and summarized
- Entries are structured and professional for hackathon submission
- Process encourages regular documentation habits
- Generated content is immediately useful for project tracking

## Example Usage Flow

1. User runs `@add-to-devlog`
2. Assistant asks 6-8 questions about the day's work
3. Assistant runs git/GitHub commands to gather technical data
4. Assistant generates structured entry and appends to DEVLOG.md
5. Assistant confirms completion and shows summary

This creates a comprehensive daily log that combines user reflection with objective technical progress tracking.