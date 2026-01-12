---
description: "Comprehensive PR review using specialized agents"
argument-hint: "<pr-number> [review-aspects]"
allowed-tools: ["Bash", "Glob", "Grep", "Read", "Task"]
---

# Comprehensive PR Review

Run a comprehensive pull request review using multiple specialized agents, each focusing on a different aspect of code quality.

**Arguments:** "$ARGUMENTS"

## Review Workflow:

1. **Determine Review Scope**
   - Parse arguments: first numeric argument is the PR number, remaining are review aspects
   - If PR number provided: fetch PR details with `gh pr view <number>`
   - If no PR number: check for existing PR on current branch with `gh pr view`
   - Identify changed files from PR diff
   - Default: Run all applicable reviews

2. **Available Review Aspects:**
   - **comments** - Analyze code comment accuracy and maintainability
   - **tests** - Review test coverage quality and completeness
   - **errors** - Check error handling for silent failures
   - **types** - Analyze type design and invariants (if new types added)
   - **code** - General code review for project guidelines
   - **simplify** - Simplify code for clarity and maintainability
   - **all** - Run all applicable reviews (default)

3. **Identify Changed Files**
   - Run `git diff --name-only` to see modified files
   - Check if PR already exists: `gh pr view`
   - Identify file types and what reviews apply

4. **Determine Applicable Reviews**

   Based on changes:
   - **Always applicable**: code-reviewer (general quality)
   - **If test files changed**: pr-test-analyzer
   - **If comments/docs added**: comment-analyzer
   - **If error handling changed**: silent-failure-hunter
   - **If types added/modified**: type-design-analyzer
   - **After passing review**: code-simplifier (polish and refine)

5. **Launch Review Agents**

   **Sequential approach** (one at a time):
   - Easier to understand and act on
   - Each report is complete before next
   - Good for interactive review

   **Parallel approach** (user can request):
   - Launch all agents simultaneously
   - Faster for comprehensive review
   - Results come back together

6. **Aggregate Results**

   After agents complete, summarize:
   - **Critical Issues** (must fix before merge)
   - **Important Issues** (should fix)
   - **Suggestions** (nice to have)
   - **Positive Observations** (what's good)

7. **Post Review to GitHub**

   **IMPORTANT**: When a PR number is provided as an argument, ALWAYS post the review summary as a GitHub PR comment using `gh pr comment <number>`.

   Use this exact format for the comment:

   ```markdown
   # PR Review Summary

   ## Critical Issues (X found)

   | Source | Issue | Location |
   |--------|-------|----------|
   | **agent-name** | Issue description | `file.ts:line` |

   ## Important Issues (X found)

   | Source | Issue | Location |
   |--------|-------|----------|
   | **agent-name** | Issue description | `file.ts:line` |

   ## Suggestions (X found)

   | Source | Suggestion | Location |
   |--------|------------|----------|
   | **agent-name** | Suggestion description | `file.ts:line` |

   ## Strengths

   - What's well-done in this PR

   ## Recommended Action

   1. Fix critical issues first
   2. Address important issues
   3. Consider suggestions
   4. Re-run review after fixes

   ---

   **Verdict**: [Summary of whether PR is ready to merge or needs changes]
   ```

   Post the comment:
   ```bash
   gh pr comment <PR_NUMBER> --body "<review_summary>"
   ```

## Usage Examples:

**Review specific PR (posts comment to GitHub):**

```
/review-pr-agents 163
# Full review of PR #163, posts summary as PR comment

/review-pr-agents 163 tests errors
# Reviews only test coverage and error handling for PR #163
```

**Review current branch's PR:**

```
/review-pr-agents
# Detects PR for current branch, runs full review

/review-pr-agents comments
# Reviews only code comments for current branch's PR
```

**Specific aspects:**

```
/review-pr-agents 42 tests errors
# Reviews only test coverage and error handling

/review-pr-agents 42 simplify
# Simplifies code after passing review
```

**Parallel review:**

```
/review-pr-agents 42 all parallel
# Launches all agents in parallel
```

## Agent Descriptions:

**comment-analyzer**:

- Verifies comment accuracy vs code
- Identifies comment rot
- Checks documentation completeness

**pr-test-analyzer**:

- Reviews behavioral test coverage
- Identifies critical gaps
- Evaluates test quality

**silent-failure-hunter**:

- Finds silent failures
- Reviews catch blocks
- Checks error logging

**type-design-analyzer**:

- Analyzes type encapsulation
- Reviews invariant expression
- Rates type design quality

**code-reviewer**:

- Checks CLAUDE.md compliance
- Detects bugs and issues
- Reviews general code quality

**code-simplifier**:

- Simplifies complex code
- Improves clarity and readability
- Applies project standards
- Preserves functionality

## Tips:

- **Run early**: Before creating PR, not after
- **Focus on changes**: Agents analyze git diff by default
- **Address critical first**: Fix high-priority issues before lower priority
- **Re-run after fixes**: Verify issues are resolved
- **Use specific reviews**: Target specific aspects when you know the concern

## Workflow Integration:

**Before committing:**

```
1. Write code
2. Run: /pr-review-toolkit:review-pr code errors
3. Fix any critical issues
4. Commit
```

**Before creating PR:**

```
1. Stage all changes
2. Run: /pr-review-toolkit:review-pr all
3. Address all critical and important issues
4. Run specific reviews again to verify
5. Create PR
```

**After PR feedback:**

```
1. Make requested changes
2. Run targeted reviews based on feedback
3. Verify issues are resolved
4. Push updates
```

## Notes:

- **Always posts to GitHub**: When reviewing a PR, the summary is automatically posted as a PR comment
- Agents run autonomously and return detailed reports
- Each agent focuses on its specialty for deep analysis
- Results are actionable with specific file:line references
- Agents use appropriate models for their complexity
- All agents available in `/agents` list
