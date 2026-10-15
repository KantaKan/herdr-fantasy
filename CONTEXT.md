# herdr fantasy

A herdr plugin that shows a repo's running coding agents as a fantasy RPG party whose animations mirror what the agents are really doing.

## Language

### Characters

**Hero**:
A persistent character that belongs to one repo, keyed by agent kind and slot; it keeps level, XP, and stats across agent sessions.
_Avoid_: Agent (when meaning the character), player

**Agent**:
A live coding-agent pane that currently plays a Hero.
_Avoid_: Hero (when meaning the process), bot

**Party**:
All Heroes currently played by Agents in the workspace.

**Class**:
A Hero's costume and role (Knight, Mage, Rogue, Ranger, Cleric, Bard), chosen from the agent kind.

**Joke Class**:
A comedy Class (e.g. 10x Engineer, Jester, Senior Dev, YAML Wizard) worn only by Summons, rolled once per Summon.

**Summon**:
A temporary character that exists while an Agent's subagent runs; it has no level of its own and its XP goes to the Hero that summoned it. It rises from shadow and sinks back into shadow when the subagent ends.
_Avoid_: Familiar, Hero (for subagents)

**Rubber Duck**:
A non-agent companion that sits by the Party's campfire and delivers Quips; it never levels and is never played by an Agent.
_Avoid_: Mascot, pet

### Story

**Quest**:
One unit of an Agent's work, from receiving a prompt until it is idle again.
_Avoid_: Task, job

**Chapter**:
The span of one agent session, made of Quests; a new session or `/clear` closes it and opens the next.
_Avoid_: Run, New Run (that's only the banner shown when a Chapter starts), session

**Boss**:
A test, build, or lint command; defeating it means the command succeeded.

**Stamina**:
A Hero's remaining context window.
_Avoid_: STA (UI label only), health

**Rest**:
Context compaction, shown as the Hero resting by the fire.
_Avoid_: Compact, sleep

**Town**:
The repo drawn as a village, one building per top-level directory.

**Quip**:
An original, text-only joke line shown in dialogue or a toast, triggered only by a real event.
_Avoid_: Meme (when meaning an image)

### Plumbing

**Stage**:
The main plugin pane showing the battle scene, Party, and HUD.

**Transcript**:
The agent's own session log, which the Stage reads to learn what the Agent is doing.
