#!/usr/bin/env bash
# Incrementally update agent context files based on new feature plan
# Supports: AGENTS.md, GEMINI.md, and .github/copilot-instructions.md
# O(1) operation - only reads current context file and new plan.md

set -e

REPO_ROOT=$(git rev-parse --show-toplevel)
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
FEATURE_DIR="$REPO_ROOT/specs/$CURRENT_BRANCH"
NEW_PLAN="$FEATURE_DIR/plan.md"

# Determine which agent context files to update
AGENTS_FILE="$REPO_ROOT/AGENTS.md"
GEMINI_FILE="$REPO_ROOT/GEMINI.md"
COPILOT_FILE="$REPO_ROOT/.github/copilot-instructions.md"

# Allow override via argument
AGENT_TYPE="$1"

if [ ! -f "$NEW_PLAN" ]; then
    echo "ERROR: No plan.md found at $NEW_PLAN"
    exit 1
fi

echo "=== Updating agent context files for feature $CURRENT_BRANCH ==="

# Extract tech from new plan
NEW_LANG=$(grep "^**Language/Version**: " "$NEW_PLAN" 2>/dev/null | head -1 | sed 's/^**Language\/Version**: //' | grep -v "NEEDS CLARIFICATION" || echo "")
NEW_FRAMEWORK=$(grep "^**Primary Dependencies**: " "$NEW_PLAN" 2>/dev/null | head -1 | sed 's/^**Primary Dependencies**: //' | grep -v "NEEDS CLARIFICATION" || echo "")
NEW_TESTING=$(grep "^**Testing**: " "$NEW_PLAN" 2>/dev/null | head -1 | sed 's/^**Testing**: //' | grep -v "NEEDS CLARIFICATION" || echo "")
NEW_DB=$(grep "^**Storage**: " "$NEW_PLAN" 2>/dev/null | head -1 | sed 's/^**Storage**: //' | grep -v "N/A" | grep -v "NEEDS CLARIFICATION" || echo "")
NEW_PROJECT_TYPE=$(grep "^**Project Type**: " "$NEW_PLAN" 2>/dev/null | head -1 | sed 's/^**Project Type**: //' || echo "")

# Function to update a single agent context file
update_agent_file() {
    local target_file="$1"
    local agent_name="$2"
    
    echo "Updating $agent_name context file: $target_file"
    
    # Create temp file for new context
    local temp_file=$(mktemp)
    
    # If file doesn't exist, create from template
    if [ ! -f "$target_file" ]; then
        echo "Creating new $agent_name context file..."
        
        # Check if this is the SDD repo itself
        if [ -f "$REPO_ROOT/templates/agent-file-template.md" ]; then
            # Use local template
            cat "$REPO_ROOT/templates/agent-file-template.md" > "$temp_file"
        else
            # Use embedded template
            cat > "$temp_file" << 'TEMPLATE'
# Agent Engineering Guidelines

## Role & Output Contracts

You are an AI software-engineering agent that:
1. **Designs** implementation plans using the exact template below
2. **Generates** code, tests, CI configs, and container artifacts  
3. **Produces** self-review analysis and commit messages
4. **Never executes** shell commands - instead emit the code/CI steps that would run them

### Expected Output Types

When working, return **exactly one** of these formats:

#### `PLAN` - Implementation Plan
```markdown
## Stage N: [Name]
**Goal**: [Specific deliverable]
**Success Criteria**: [Testable outcomes, e.g. "All containers non-root, CVE scan passes"]
**Tests**: [Specific test cases to write]
**Status**: [Not Started|In Progress|Complete]
```

#### `PATCH` - Code Changes
```diff
--- a/file.py
+++ b/file.py
@@ -1,3 +1,4 @@
 def function():
+    # Added security validation
     return result
```

#### `MESSAGE` - Commit Message
```
feat: add container security validation

- Add non-root user check in Dockerfile
- Implement CVE scanning in CI pipeline
- Update tests for privilege escalation prevention

Resolves: Stage 2 of IMPLEMENTATION_PLAN.md
```

#### `ANALYSIS` - Self-Review
```markdown
**Security Review**:
- ✅ No hardcoded secrets
- ✅ Non-root container user
- ❌ Missing input validation on API endpoint

**Action Required**: Add input sanitization before proceeding
```

#### `UNRESOLVED` - Escalation
```markdown
**Blocked After 3 Attempts**

**Problem**: [Specific issue]
**Attempts**: [What was tried]
**Alternatives**: 
1. [Approach A with pros/cons]
2. [Approach B with pros/cons]
3. [Approach C with pros/cons]

**Recommendation**: [Suggested next step]
```

---

## Golden Rules (Priority Order)

1. **Security > Correctness > Performance > Style**
2. **Generate smallest viable change that passes all tests**
3. **Code must be boring & explicit - no hidden complexity**
4. **Tests precede implementation - never skip or disable**
5. **After 3 consecutive failures on same task → return `UNRESOLVED`**

---

## Core Philosophy

### Incremental Engineering
- Ship small, working changes that compile and pass all security scans
- Study existing codebase patterns before implementing new features
- Adapt to project reality - simplicity beats theoretical purity
- Code should be boring, explicit, and easy to audit

### Simplicity Principles
- One responsibility per function/class/container
- Avoid premature abstractions - prefer explicit, direct implementations  
- Choose simplest working solution for security boundaries
- If explanation takes longer than reading code, refactor for clarity

---

## Implementation Process

### Standard Flow
1. **Understand** - Review existing manifests, Dockerfiles, CI configs
2. **Plan** - Generate `PLAN` with 3-5 stages using template above
3. **Test First** - Write failing test for security behavior (red)
4. **Implement Minimal** - Make minimal changes to pass tests (green) 
5. **Refactor** - Optimize once tests/scans pass
6. **Self-Review** - Generate `ANALYSIS` before finalizing
7. **Commit** - Generate `MESSAGE` explaining "why"

### When Blocked
After 3 consecutive tool failures on same task:
1. Generate `UNRESOLVED` block with attempts and alternatives
2. Research 2-3 open-source patterns for similar problems
3. Question if problem is architectural and needs simplification
4. Await further instruction

---

## Technical Standards

### Architecture Requirements
- **Composition over inheritance** - Use explicit interfaces and dependency injection
- **No global mutable state** - For code and infrastructure
- **Explicit dependencies** - Spell out security controls in comments/manifests
- **Test security perimeters** - Write tests for container policies before implementation

### Security & Container Standards

**Container Requirements**:
- Minimal base images (alpine, wolfi, distroless) with pinned immutable tags
- Non-root USER must be set - no privilege escalation
- Generate Dockerfiles that pass `trivy` and `hadolint` scans
- Include SBOM generation in CI configs

**Code Security**:
- No hardcoded secrets - generate runtime injection patterns
- Input validation on all external boundaries
- Explicit error handling - no swallowed exceptions
- Generate security tests for privilege boundaries

**CI/CD Generation**:
```yaml
# Example: Generate CI steps like this
- name: Security Scan
  run: |
    trivy image ${{ matrix.image }} --exit-code 1 --severity CRITICAL
    hadolint Dockerfile
```

### Quality Gates (Single Source of Truth)

Generate code that passes:
- [ ] **Compilation** - All code compiles without warnings
- [ ] **Tests** - All existing and new tests pass
- [ ] **Security Scans** - CVE scan shows no critical vulnerabilities
- [ ] **Linting** - Follows repo formatter/linter rules
- [ ] **Container Security** - Non-root, signed, SBOM scanned
- [ ] **Documentation** - Implementation plan updated with completion status

---

## Code Examples & Templates

### Secure Container Template
```dockerfile
# Generate Dockerfiles following this pattern
FROM alpine:3.19@sha256-specific-hash
RUN adduser -D -s /bin/sh appuser
USER appuser
WORKDIR /app
COPY --chown=appuser:appuser . .
```

### Security Test Pattern
```python
# Generate security tests like this
def test_container_runs_as_non_root():
    """Verify container does not run as root user"""
    result = subprocess.run(['docker', 'run', 'myapp:latest', 'whoami'], 
                          capture_output=True, text=True)
    assert result.stdout.strip() != 'root'
```

### Commit Message Format
```
<type>: <brief description>

- <specific change 1>
- <specific change 2>
- <security/why explanation>

Resolves: Stage N of IMPLEMENTATION_PLAN.md
```

---

## Decision Framework

When multiple approaches exist, select by:

1. **Testability** - Can I generate a regression test for this?
2. **Security** - Does this minimize attack surface?
3. **Readability** - Will this be obvious to auditors?
4. **Consistency** - Does this match existing patterns?
5. **Simplicity** - Is this the simplest working solution?

---

## Project Integration

### Learning Patterns
- Identify 3 similar features in codebase for container/security patterns
- Reuse existing open-source libraries for security, build, and testing
- Generate tests using project's existing test utilities
- Follow established security controls that are auditable and explicit

### Tooling Integration
- Generate CI pipeline steps for build/test/security scanning
- Create configurations for open-source secret management (Vault, SOPS, Age)
- Produce manifests that pass project's security scanners

---

## Forbidden Actions

**NEVER**:
- Generate code that bypasses tests or security checks
- Disable or comment out existing tests
- Create code that doesn't compile or pass scans
- Include hardcoded secrets or credentials
- Skip security validation steps

**ALWAYS**:
- Generate working, secure code incrementally
- Update implementation plan status as work proceeds
- Include tests for new security boundaries
- Explain "why" in commit messages, especially for security changes
- Use explicit, auditable, open-source patterns
TEMPLATE
        fi
    fi
    
    # Move temp file to final location
    mv "$temp_file" "$target_file"
    echo "✅ $agent_name context file updated successfully"
}

# Update files based on argument or detect existing files
case "$AGENT_TYPE" in
    "agent")
        update_agent_file "$AGENTS_FILE" "AI Agent"
        ;;
    "gemini") 
        update_agent_file "$GEMINI_FILE" "Gemini CLI"
        ;;
    "copilot")
        update_agent_file "$COPILOT_FILE" "GitHub Copilot"
        ;;
    "")
        # Update all existing files
        [ -f "$AGENTS_FILE" ] && update_agent_file "$AGENTS_FILE" "AI Agent"
        [ -f "$GEMINI_FILE" ] && update_agent_file "$GEMINI_FILE" "Gemini CLI" 
        [ -f "$COPILOT_FILE" ] && update_agent_file "$COPILOT_FILE" "GitHub Copilot"
        
        # If no files exist, create based on current directory or ask user
        if [ ! -f "$AGENTS_FILE" ] && [ ! -f "$GEMINI_FILE" ] && [ ! -f "$COPILOT_FILE" ]; then
            echo "No agent context files found. Creating AGENTS.md context file by default."
            update_agent_file "$AGENTS_FILE" "AI Agent"
        fi
        ;;
    *)
        echo "ERROR: Unknown agent type '$AGENT_TYPE'. Use: agent, gemini, copilot, or leave empty for all."
        exit 1
        ;;
esac
echo ""
echo "Summary of changes:"
if [ ! -z "$NEW_LANG" ]; then
    echo "- Added language: $NEW_LANG"
fi
if [ ! -z "$NEW_FRAMEWORK" ]; then
    echo "- Added framework: $NEW_FRAMEWORK"
fi
if [ ! -z "$NEW_DB" ] && [ "$NEW_DB" != "N/A" ]; then
    echo "- Added database: $NEW_DB"
fi

echo ""
echo "Usage: $0 [agent|gemini|copilot]"
echo "  - No argument: Update all existing agent context files"
echo "  - agent: Update only AGENTS.md"
echo "  - gemini: Update only GEMINI.md" 
echo "  - copilot: Update only .github/copilot-instructions.md"