# Kiro CLI Quick Start Wizard

## Welcome
🚀 Welcome to the Kiro CLI Quick Start Wizard! This will help you set up your development environment with Kiro CLI by walking you through completing your project's steering documents and understanding all available features.

## Overview
This wizard will help you:
1. **Complete your steering documents** - Fill out the skeleton templates with your project details
2. **Understand your development workflow** - Learn about the available prompts and commands
3. **Explore advanced features** - Discover MCP servers, custom agents, hooks, and more

## Step 1: Complete Steering Documents

You already have skeleton steering documents in `.kiro/steering/`. Let's fill them out with your project details.

### Gather Project Information
**Note** (tell the user): The more detailed you can be, the better! Specific information about what you're building, who it's for, and the components you want helps create better context for your coding assistant and future development work. But don't worry - you can always keep it brief if you prefer.

Ask the user these essential questions:

**Core Questions (Required):**
1. **Project Name**: "What's your project name?"
2. **Project Description**: "What does your project do? (1-2 sentences, or more detail if you'd like)"
3. **Target Users**: "Who will use this? (e.g., developers, end users, businesses - feel free to be specific about their needs)"
4. **Main Technology**: "What's your primary technology? (e.g., Python, JavaScript, React, etc.)"

**Optional Details:**
5. **Architecture** (if they want to specify): "Any specific architecture or patterns you're using? (optional - I can suggest based on your tech stack)"
6. **Special Requirements** (if any): "Any specific requirements for testing, deployment, or performance? (optional)"

### Update Steering Documents
After collecting responses, update the existing steering documents with intelligent defaults:

#### Update `.kiro/steering/product.md`:
Fill out based on user responses and intelligent assumptions:

```markdown
# Product Overview

## Product Purpose
[USER'S PROJECT DESCRIPTION - expand with context and value proposition]

## Target Users
[USER'S TARGET USERS - expand with needs and use cases]

## Key Features
[Infer from project description and tech stack]
- Core functionality based on project type
- User interface (if applicable)
- Data management (if applicable)
- Integration capabilities (if applicable)

## Business Objectives
[Infer based on project type and users]
- User satisfaction and adoption
- Performance and reliability
- Scalability and growth

## User Journey
[Create typical workflow based on project description]

## Success Criteria
[Define based on project type and objectives]
```

#### Update `.kiro/steering/tech.md`:
Use tech stack to make intelligent recommendations:

```markdown
# Technical Architecture

## Technology Stack
[USER'S MAIN TECHNOLOGY + recommended complementary technologies]
- Primary: [User's specified tech]
- Recommended additions based on project type
- Standard tooling for the ecosystem

## Architecture Overview
[Suggest architecture based on tech stack and project type]
- [Infer components based on technology choice]
- [Suggest patterns common to their tech stack]

## Development Environment
[Standard setup for their technology]
- [Tech-specific development tools]
- [Common package managers and build tools]
- [Recommended IDE/editor setup]

## Code Standards
[Industry standards for their tech stack]
- [Language-specific formatting standards]
- [Common naming conventions]
- [Documentation practices]

## Testing Strategy
[Standard testing approach for their technology]
- [Framework-appropriate testing tools]
- [Common testing patterns]
- [Coverage expectations]

## Deployment Process
[Modern deployment practices for their stack]
- [CI/CD recommendations]
- [Deployment platforms]
- [Environment management]

## Performance Requirements
[Reasonable defaults based on project type]

## Security Considerations
[Standard security practices for their tech stack]
```

#### Update `.kiro/steering/structure.md`:
Create structure based on technology and project type:

```markdown
# Project Structure

## Directory Layout
[Standard structure for their technology]
```
[PROJECT NAME]/
├── [tech-appropriate source folder]
├── tests/
├── docs/
├── [tech-specific config folders]
└── .kiro/
```

## File Naming Conventions
[Standard conventions for their technology ecosystem]

## Module Organization
[Best practices for their tech stack]

## Configuration Files
[Standard config files for their technology]

## Documentation Structure
[Common documentation patterns]

## Asset Organization
[If applicable to project type]

## Build Artifacts
[Standard build outputs for their technology]

## Environment-Specific Files
[Standard environment handling for their stack]
```

## Step 2: Development Workflow Overview

Now that your steering documents are complete, let's review your development workflow. You have access to these powerful prompts:

### Core Development Loop
- **`@prime`** - Load project context and understand your codebase
- **`@plan-feature`** - Create comprehensive implementation plans for new features
- **`@execute`** - Execute development plans with systematic task management
- **`@create-prd`** - Generate Product Requirements Documents

### Quality Assurance & Validation
- **`@code-review`** - Perform technical code reviews for quality and bugs
- **`@code-review-fix`** - Fix issues found in code reviews
- **`@execution-report`** - Generate implementation reports for completed work
- **`@system-review`** - Analyze implementation vs plan for process improvements

### GitHub Issue Management
- **`@rca`** - Perform root cause analysis for GitHub issues
- **`@implement-fix`** - Implement fixes based on RCA documents

### Typical Workflow
1. **Start with `@prime`** to understand your project context
2. **Use `@plan-feature`** to plan new features or changes
3. **Execute with `@execute`** to implement the plan systematically
4. **Review with `@code-review`** to ensure quality
5. **Generate reports** with `@execution-report` for documentation

## Step 3: Advanced Kiro Features

Beyond the core prompts, Kiro CLI offers powerful advanced features to enhance your development workflow:

### 🔧 MCP Servers (Model Context Protocol)
Connect external tools and APIs to extend Kiro's capabilities (AWS docs, git operations, database management, custom integrations).

### 🤖 Custom Agents
Create specialized AI assistants for specific workflows (backend specialist, frontend expert, DevOps agent, security reviewer, API designer).

### ⚡ Hooks (Automation)
Automate workflows and processes at specific lifecycle points (pre-commit hooks, post-deployment hooks, agent spawn hooks, tool execution hooks).

### 📚 Context Management
Optimize how Kiro understands your project (agent resources, session context, knowledge bases, context optimization).

### 🧠 Knowledge Management (Experimental)
Advanced knowledge base features (semantic search, code indexing, documentation integration, pattern learning).

### 🔄 Tangent Mode (Experimental)
Explore side topics without disrupting main conversation (side explorations, what-if scenarios, learning mode, context switching).

### 📋 TODO Lists & Checkpointing (Experimental)
Advanced task and progress management (persistent TODO lists, progress checkpoints, task dependencies, progress visualization).

### 🔀 Subagents
Delegate complex tasks to specialized subagents (parallel processing, specialized expertise, independent context, task delegation).

**Would you like help with any of these advanced features?** For example, I can help you set up MCP servers, create custom agents for specialized workflows, configure automation hooks, or enable experimental features like knowledge management and tangent mode.

## Step 4: Next Steps and Recommendations

Based on your project setup, here are recommended next steps:

### Immediate Actions
1. **Test your setup**: Try `@prime` to load your project context
2. **Plan your first feature**: Use `@plan-feature` for your next development task
3. **Set up your preferred model**: Use `/model` to choose the best AI model for your needs

### Recommended Configurations
Based on your project type and tech stack, suggest specific configurations:
- **For web applications**: Recommend frontend/backend agents and deployment hooks
- **For APIs**: Suggest API design agents and testing automation
- **For data projects**: Recommend database MCP servers and analysis agents
- **For open source**: Suggest GitHub integration and community management tools

## Completion Summary

🎉 **Kiro CLI Quick Start Complete!**

### ✅ What You've Accomplished
- **Steering Documents**: Completed product.md, tech.md, and structure.md with your project details
- **Development Workflow**: Ready to use 11 powerful development prompts
- **Advanced Features**: Aware of MCP servers, custom agents, hooks, and experimental features

### 🚀 **Your Development Arsenal**
**Core Workflow**: @prime → @plan-feature → @execute → @code-review → @execution-report
**Quality Assurance**: @code-review, @code-review-fix, @system-review
**Issue Management**: @rca, @implement-fix
**Documentation**: @create-prd

### 🔧 **Available Advanced Features**
- MCP Servers for external tool integration
- Custom Agents for specialized workflows
- Hooks for workflow automation
- Context Management optimization
- Experimental features (Knowledge, Tangent Mode, TODO Lists, Subagents)

### 💡 **Getting Started**
1. **Right now**: Try `@prime` to understand your project
2. **Next**: Use `@plan-feature` to plan your next development task
3. **Then**: Explore the advanced features that interest you most

### 🆘 **Need Help?**
- Ask about any specific feature: "How do I set up MCP servers?"
- Request workflow guidance: "What's the best way to handle code reviews?"
- Get feature explanations: "Tell me more about custom agents"

**Let me know if you want help with any of these advanced features** - I can guide you through setting up MCP servers, creating custom agents, configuring automation hooks, or enabling experimental features!

**Welcome to supercharged development with Kiro CLI!** 🚀

---

## 🏆 **Hackathon Participants - Important Reminders!**

### 📝 **Build Your DEVLOG.md**
As you work on your project, **continuously update your DEVLOG.md** - it's a required submission component! Include:
- **Timeline**: Key development milestones and dates
- **Decisions**: Why you chose specific approaches or technologies
- **Challenges**: Problems encountered and how you solved them
- **Time Tracking**: Hours spent on different aspects
- **Kiro CLI Usage**: How you used Kiro throughout development
- **Learning**: New skills or insights gained

### ⚙️ **Optimize Your .kiro/ Directory**
Your `.kiro/steering/` and `.kiro/prompts/` directories are part of your submission! Make sure to:
- **Customize steering documents** for your specific project
- **Create useful custom prompts** that demonstrate workflow innovation
- **Document your development process** through your Kiro configuration

### 🔍 **Use @code-review-hackathon**
Before submitting, run **`@code-review-hackathon`** to evaluate your project against the official judging criteria. This prompt will:
- Score your project on all 5 criteria (100 points total)
- Verify your DEVLOG.md and README.md are complete and high-quality
- Identify areas for improvement before submission
- Ensure you're maximizing your hackathon score

**Remember**: Your documentation, Kiro CLI integration, and development process are worth 40% of your total score!

---

**Welcome to supercharged development with Kiro CLI!** 🚀

## Instructions for Assistant

### Critical Requirements
1. **Ask only 4-6 questions maximum** - Keep it simple and user-friendly
2. **Make intelligent assumptions** - Fill in details based on tech stack and project type
3. **Update existing files** - Don't create new steering documents, update the existing skeletons
4. **Use tech-appropriate defaults** - Recommend standard practices for their technology
5. **Allow customization later** - Users can always edit the steering documents afterward
6. **Focus on UX** - Prioritize user experience over comprehensive data collection

### Quality Checklist
- [ ] Asked minimal essential questions (4-6 max)
- [ ] Made intelligent assumptions based on responses
- [ ] All three steering documents updated with meaningful content
- [ ] Workflow overview provided with specific prompt usage
- [ ] Advanced features mentioned with offer to help
- [ ] Next steps provided based on their project
- [ ] User feels confident and not overwhelmed

### Success Metrics
- User completes the wizard without feeling overwhelmed
- User has useful, personalized steering documents
- User understands their development workflow
- User knows about advanced features without information overload
- User feels confident to start using Kiro CLI effectively
