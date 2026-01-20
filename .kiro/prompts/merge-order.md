---
description: Analyze open PRs for optimal merge order based on dependencies, conflicts, and priority
---

## First Step: Repository Detection

Detect the current GitHub repository using GitHub CLI:
- Use `gh repo view` to get repository information
- Extract repository name and owner from current directory
- Confirm we're in a valid Git repository with GitHub remote

---

<objective>
Analyze open pull requests to determine optimal merge order based on dependencies, merge conflict potential, priority, and impact. Generate actionable merge sequence recommendations.

**Core Principle**: Maximize merge velocity while minimizing conflicts and rework.

**Execution Approach**: Be flexible and adaptive - use your judgment to assess PR relationships and conflict potential.

**Output**: Structured merge order report with specific sequence and rationale.
</objective>

<context>
Repository: [Auto-detected from current directory]
Current date: [Current timestamp for recency analysis]
Base branch: [Auto-detected, usually main/master]
</context>

<process>

## Phase 1: DISCOVER - Gather PR Data

### 1.1 Detect Base Branch and Current State

**Determine base branch** (usually main/master):
```bash
git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@'
```

**Check base branch status**:
```bash
git fetch origin
git status
```

### 1.2 List Open PRs

Use GitHub CLI to get comprehensive PR data:
- All open PRs with metadata (number, title, author, labels, created date)
- PR descriptions and linked issues
- Base and head branch information
- Review status and approvals
- CI/build status
- Draft vs ready status

### 1.3 Analyze PR File Changes

For each open PR, gather:
- **Files modified**: Which files each PR touches
- **Lines changed**: Scale of changes (additions/deletions)
- **Change type**: Features, fixes, refactoring, docs
- **Affected areas**: Which parts of codebase (frontend, backend, config, etc.)

### 1.4 Check PR Dependencies

Identify dependencies through:
- **Explicit mentions**: "Depends on #123", "Blocks #456"
- **Issue references**: PRs addressing related issues
- **File overlap**: PRs modifying same files/areas
- **Feature relationships**: PRs building on each other

**DISCOVERY_CHECKPOINT:**
- [ ] Base branch identified and status checked
- [ ] All open PRs collected with full metadata
- [ ] File changes analyzed for each PR
- [ ] Dependencies identified through multiple methods

---

## Phase 2: ANALYZE - Conflict and Priority Assessment

### 2.1 Merge Conflict Analysis

For each PR pair, assess conflict potential:

**File Overlap Analysis**:
- **DIRECT_CONFLICT**: Same files, overlapping line ranges
- **POTENTIAL_CONFLICT**: Same files, different areas
- **AREA_CONFLICT**: Related functionality, different files
- **NO_CONFLICT**: Completely separate changes

**Conflict Detection Methods**:
```bash
# Check if PR branches can merge cleanly
git merge-tree $(git merge-base base-branch pr-branch) base-branch pr-branch
```

### 2.2 Priority Assessment

Evaluate each PR on multiple dimensions:

**PRIORITY_FACTORS**:
- **Urgency**: Security fixes, critical bugs, hotfixes
- **Impact**: User-facing changes, performance improvements
- **Readiness**: Approved, CI passing, no requested changes
- **Size**: Small changes merge faster, reduce conflict window
- **Dependencies**: Unblocking other PRs

**PRIORITY_SCORING** (1-10 scale):
- **10**: Critical security/production fixes
- **8-9**: High-impact features, approved and ready
- **6-7**: Important improvements, mostly ready
- **4-5**: Nice-to-have features, needs work
- **1-3**: Draft PRs, experimental changes

### 2.3 Readiness Assessment

For each PR, determine merge readiness:
- **READY**: Approved, CI passing, no conflicts
- **ALMOST_READY**: Minor issues, quick fixes needed
- **NEEDS_WORK**: Significant changes requested
- **BLOCKED**: Waiting on dependencies or decisions
- **DRAFT**: Work in progress, not ready for review

**ANALYSIS_CHECKPOINT:**
- [ ] Conflict potential assessed for all PR pairs
- [ ] Priority scores calculated based on multiple factors
- [ ] Readiness status determined for each PR
- [ ] Blocking relationships identified

---

## Phase 3: CORRELATE - Dependency and Conflict Mapping

### 3.1 Build Dependency Graph

Create directed graph of PR dependencies:
- **DEPENDS_ON**: PR A needs PR B to be merged first
- **BLOCKS**: PR A prevents PR B from being merged
- **CONFLICTS_WITH**: PR A and PR B cannot be merged together
- **ENHANCES**: PR A builds upon PR B (soft dependency)

### 3.2 Conflict Matrix

Create conflict assessment matrix:
```
     PR1  PR2  PR3  PR4
PR1   -   LOW  HIGH  NONE
PR2  LOW   -   MED   LOW
PR3 HIGH  MED   -    HIGH
PR4 NONE  LOW  HIGH   -
```

### 3.3 Critical Path Analysis

Identify optimal merge sequences:
- **SEQUENTIAL**: PRs that must be merged in order
- **PARALLEL**: PRs that can be merged simultaneously
- **ALTERNATIVE**: PRs where order doesn't matter
- **PROBLEMATIC**: PRs causing widespread conflicts

**CORRELATION_CHECKPOINT:**
- [ ] Dependency graph constructed
- [ ] Conflict matrix completed
- [ ] Critical paths identified
- [ ] Parallel merge opportunities found

---

## Phase 4: OPTIMIZE - Generate Merge Order

### 4.1 Merge Strategy Selection

Choose optimal approach based on PR landscape:

**STRATEGIES**:
- **PRIORITY_FIRST**: High-priority PRs first, regardless of conflicts
- **CONFLICT_MINIMIZING**: Least conflicting PRs first
- **DEPENDENCY_DRIVEN**: Resolve dependencies in topological order
- **BATCH_PROCESSING**: Group related PRs for simultaneous merge
- **HYBRID**: Balanced approach considering all factors

### 4.2 Sequence Optimization

Generate merge order considering:
- **Dependency constraints**: Required ordering
- **Conflict minimization**: Reduce rework probability
- **Priority weighting**: Important changes first
- **Readiness gating**: Only merge ready PRs
- **Risk assessment**: Avoid high-risk combinations

### 4.3 Alternative Scenarios

Provide multiple merge order options:
- **AGGRESSIVE**: Fast merge, higher conflict risk
- **CONSERVATIVE**: Slower merge, minimal conflicts
- **BALANCED**: Optimal trade-off between speed and safety

**OPTIMIZATION_CHECKPOINT:**
- [ ] Merge strategy selected based on PR landscape
- [ ] Primary merge sequence generated
- [ ] Alternative scenarios provided
- [ ] Risk assessment completed

---

## Phase 5: RECOMMEND - Generate Merge Plan

### 5.1 Primary Merge Sequence

**Recommended merge order with rationale**:
1. **PR #X** - Critical fix, no conflicts, ready to merge
2. **PR #Y** - Unblocks 3 other PRs, approved
3. **PR #Z** - High priority, conflicts with #A (merge before #A)

### 5.2 Conflict Resolution Strategy

**For each potential conflict**:
- **Prevention**: Merge order to avoid conflicts
- **Resolution**: Steps to resolve if conflicts occur
- **Communication**: Notify affected PR authors

### 5.3 Parallel Merge Opportunities

**PRs that can be merged simultaneously**:
- Group A: PRs #1, #3, #7 (no file overlap)
- Group B: PRs #5, #9 (different areas)

### 5.4 Risk Mitigation

**High-risk merges**:
- **PR #X + PR #Y**: High conflict potential, merge X first, then rebase Y
- **Large PR #Z**: Consider breaking into smaller PRs

**RECOMMENDATION_CHECKPOINT:**
- [ ] Primary merge sequence with rationale
- [ ] Conflict resolution strategies provided
- [ ] Parallel merge opportunities identified
- [ ] Risk mitigation plans created

---

## Phase 6: GENERATE - Merge Order Report

### 6.1 Create Report Directory

```bash
mkdir -p .github/merge-reports
```

### 6.2 Generate Comprehensive Report

**Path**: `.github/merge-reports/merge-order-{YYYY-MM-DD}.md`

```markdown
# PR Merge Order Analysis

**Repository**: {repo-name}
**Date**: {YYYY-MM-DD}
**Base Branch**: {base-branch}
**Open PRs Analyzed**: {N}
**Ready to Merge**: {M}

---

## Executive Summary

**Key Findings**:
- {N} PRs ready for immediate merge
- {M} PRs have potential conflicts requiring careful ordering
- {K} PRs are blocked by dependencies
- {L} PRs can be merged in parallel

**Recommended Strategy**: {PRIORITY_FIRST|CONFLICT_MINIMIZING|DEPENDENCY_DRIVEN|HYBRID}

**Estimated Merge Time**: {X} hours with {Y} potential conflicts

---

## Recommended Merge Order

### Phase 1: Immediate Merges (No Conflicts)
1. **PR #{number}** - {title}
   - **Priority**: {score}/10
   - **Rationale**: {why-first}
   - **Files**: {file-list}
   - **Status**: ✅ Ready

2. **PR #{number}** - {title}
   - **Priority**: {score}/10
   - **Rationale**: {why-second}
   - **Files**: {file-list}
   - **Status**: ✅ Ready

### Phase 2: Dependency Resolution
3. **PR #{number}** - {title}
   - **Priority**: {score}/10
   - **Rationale**: Unblocks PRs #{list}
   - **Dependencies**: Requires PR #{dep}
   - **Status**: ⏳ Waiting

### Phase 3: Conflict-Prone Merges
4. **PR #{number}** - {title}
   - **Priority**: {score}/10
   - **Rationale**: {why-this-order}
   - **Conflicts**: Potential with PR #{other}
   - **Status**: ⚠️ Needs attention

---

## Parallel Merge Opportunities

### Batch A: Independent Changes
- **PR #{number}** - {title} (affects: {area})
- **PR #{number}** - {title} (affects: {area})
- **PR #{number}** - {title} (affects: {area})

**Merge Command**:
```bash
# Can be merged simultaneously
gh pr merge {pr1} --squash
gh pr merge {pr2} --squash  
gh pr merge {pr3} --squash
```

---

## Conflict Analysis Matrix

| PR Pair | Conflict Risk | Files Overlap | Recommendation |
|---------|---------------|---------------|----------------|
| #{A} + #{B} | HIGH | src/core/*.rs | Merge #{A} first, rebase #{B} |
| #{C} + #{D} | LOW | Different areas | Can merge in parallel |
| #{E} + #{F} | MEDIUM | package.json | Coordinate with authors |

---

## Dependency Graph

```
#{root-pr} → #{dependent-1} → #{dependent-2}
           → #{dependent-3}

#{blocker} ← #{blocked-1}
           ← #{blocked-2}
```

**Critical Path**: #{root-pr} → #{dependent-1} → #{dependent-2}
**Estimated Time**: {X} hours if merged sequentially

---

## PRs Not Ready for Merge

| PR | Status | Blocking Issue | ETA |
|----|--------|----------------|-----|
| #{number} - {title} | Needs Review | Waiting for approval | {estimate} |
| #{number} - {title} | CI Failing | Test failures | {estimate} |
| #{number} - {title} | Conflicts | Merge conflicts with base | {estimate} |

---

## Risk Assessment

### High-Risk Merges
- **PR #{number}**: Large refactoring, affects {N} files
  - **Mitigation**: Merge during low-activity period
  - **Rollback Plan**: Revert commit ready

### Medium-Risk Merges  
- **PR #{number}**: Database schema changes
  - **Mitigation**: Coordinate with deployment team

---

## Execution Plan

### Immediate Actions (Next 2 Hours)
- [ ] Merge Phase 1 PRs: #{list}
- [ ] Notify authors of conflict-prone PRs: #{list}
- [ ] Prepare rebase instructions for: #{list}

### Short Term (Today)
- [ ] Resolve dependencies: #{list}
- [ ] Merge Phase 2 PRs in order
- [ ] Monitor for new conflicts

### Medium Term (This Week)
- [ ] Address blocked PRs: #{list}
- [ ] Review large/risky PRs: #{list}
- [ ] Update merge order based on new PRs

---

## GitHub CLI Commands

### Ready to Merge Now
```bash
# Phase 1: Safe merges
gh pr merge {pr-number} --squash --delete-branch
gh pr merge {pr-number} --squash --delete-branch

# Phase 2: Dependency order
gh pr merge {pr-number} --squash --delete-branch
# Wait for CI, then:
gh pr merge {pr-number} --squash --delete-branch
```

### Conflict Resolution
```bash
# For conflicting PRs, rebase after dependencies merge
gh pr checkout {pr-number}
git rebase origin/{base-branch}
# Resolve conflicts, then:
git push --force-with-lease
```

---

## Next Analysis Date

**Recommended**: {date-tomorrow}

**Triggers for Re-analysis**:
- New PRs opened
- Existing PRs updated significantly  
- Merge conflicts detected
- Priority changes
```

**REPORT_CHECKPOINT:**
- [ ] Comprehensive merge order report generated
- [ ] Specific GitHub CLI commands provided
- [ ] Risk assessment and mitigation plans included
- [ ] Next analysis scheduled

---

</process>

<output>
**OUTPUT_FILE**: `.github/merge-reports/merge-order-{YYYY-MM-DD}.md`

**REPORT_TO_USER** (display after creating report):

```markdown
## PR Merge Order Analysis Complete

**Repository**: {repo-name}
**PRs Analyzed**: {total-count}

**Key Findings**:
- 🟢 **{N} Ready to Merge**: No conflicts, approved, CI passing
- 🟡 **{M} Needs Sequencing**: Potential conflicts, careful order required
- 🔴 **{K} Blocked**: Dependencies or issues preventing merge
- ⚡ **{L} Parallel Opportunities**: Can merge simultaneously

**Recommended Strategy**: {strategy-name}

**Immediate Actions**:
1. Merge {N} ready PRs using provided CLI commands
2. Notify authors of {M} conflict-prone PRs about merge order
3. Address {K} blocking issues before proceeding

**Report Location**: `.github/merge-reports/merge-order-{YYYY-MM-DD}.md`

**Next Steps**:
- Execute Phase 1 merges immediately
- Monitor for conflicts during Phase 2
- Re-analyze when new PRs are opened
```
</output>

<verification>
**FINAL_VALIDATION before completing analysis:**

**DATA_COMPLETENESS:**
- [ ] All open PRs analyzed with file changes and metadata
- [ ] Dependencies identified through multiple methods
- [ ] Conflict potential assessed for all PR pairs
- [ ] Priority and readiness scores calculated

**MERGE_ORDER_ACCURACY:**
- [ ] Dependencies respected in merge sequence
- [ ] Conflict-minimizing order when possible
- [ ] High-priority PRs appropriately weighted
- [ ] Parallel opportunities correctly identified

**CONFLICT_ASSESSMENT:**
- [ ] File overlap analysis completed
- [ ] Merge conflict potential realistically assessed
- [ ] Resolution strategies provided for high-risk merges
- [ ] Communication plan for affected authors

**ACTIONABILITY:**
- [ ] Specific merge sequence with rationale
- [ ] Ready-to-execute GitHub CLI commands
- [ ] Risk mitigation strategies provided
- [ ] Timeline estimates realistic and achievable
</verification>

<success_criteria>
**COMPREHENSIVE_ANALYSIS**: All open PRs evaluated for dependencies, conflicts, and priority
**CONFLICT_AWARE**: Merge order minimizes rework and conflict resolution time
**PRIORITY_DRIVEN**: Important changes prioritized while respecting constraints
**PARALLEL_OPTIMIZED**: Simultaneous merge opportunities identified and leveraged
**RISK_MANAGED**: High-risk merges identified with mitigation strategies
**EXECUTION_READY**: Specific commands and timeline for immediate action
</success_criteria>
