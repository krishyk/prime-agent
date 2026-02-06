# 100% vibe coded

i have not looked at ANY code.  I have not looked at any tests, i have no idea what the hell is happening here.  Just accept AI Love into your heart and die from the greatest increase of productivity ever

# prime-agent

Skill-driven AGENTS.md builder and synchronizer for managing reusable
instructions stored as Markdown files.

## Commands

- `prime-agent get <skill1,skill2,...>`: Build `AGENTS.md` from skills.
- `prime-agent set <name> <path>`: Store a skill file as `skills/<name>/SKILL.md`.
- `prime-agent sync`: Two-way sync between `AGENTS.md` and skills.
- `prime-agent delete <name>`: Remove a skill section from `AGENTS.md`.
- `prime-agent delete-globally <name>`: Remove section and skill file.

## Skills Directory

- Default: `./skills`
- Override with `--skills-dir` or `PRIME_AGENT_SKILLS_DIR`.
