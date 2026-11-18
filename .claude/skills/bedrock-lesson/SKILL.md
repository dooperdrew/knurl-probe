---
name: bedrock-lesson
description: Share a lesson with every DooperDrew project through bedrock. Use when you hit a problem that another repo could hit too - a CI failure on main, a recurring bug class, a tool, platform or dependency gotcha, a review finding that deserves a rule - and before you finish the task where it happened. Not for problems specific to this project only.
---

# Share a lesson with bedrock

bedrock is the shared kit of all DooperDrew repos. A lesson filed there becomes a rule with a check and reaches every repo through a sync PR, so the other projects learn it without anyone having to remember it.

1. **Is it general?** It must be able to happen in another repo with the same stack (`rust`, `node`) or in any repo (`all`). If it only concerns this project, write it in this repo's own CLAUDE.md section instead (outside the bedrock block) and stop.

2. **Look for duplicates**:
   ```bash
   gh issue list -R dooperdrew/bedrock --label lesson --state all --search "<keywords>"
   ```
   If one exists, add a comment with the new evidence (`gh issue comment <n> -R dooperdrew/bedrock --body-file <file>`): a second occurrence raises its priority.

3. **File it**: write the body to a temporary file, then
   ```bash
   gh issue create -R dooperdrew/bedrock --label lesson --title "lesson: <what to do, in a few words>" --body-file <file>
   ```
   Body, in this order:
   - **What happened**: repo, and a link to the commit, PR, CI run or file.
   - **Root cause**: why it happened, in one or two sentences.
   - **Proposed rule**: one sentence, written as an instruction.
   - **How to check it automatically**: the CI step, lint, test or script that would catch it. If you cannot think of one, say so.
   - **Stacks**: `all`, `rust` or `node`.

   **In a cloud session** (`CLAUDE_CODE_REMOTE=true`) the GitHub proxy refuses `gh issue` (GraphQL). Use REST instead:
   ```bash
   gh api repos/dooperdrew/bedrock/issues --jq .html_url -f title="lesson: <...>" -F body=@<file> -f "labels[]=lesson"
   ```
   (search: `gh api "search/issues?q=repo:dooperdrew/bedrock+label:lesson+<keywords>"`). If bedrock is not reachable from the session (403 or 404: only the session's own repo is attached), put the lesson in your PR body under a `## bedrock lesson` heading, with the same sections: the nightly task reads PR bodies and files it.

4. Never put secrets, tokens, personal data or private customer data in the issue.

5. Put the issue link in your final report to the user.

File the issue now, in the same task: it only needs `gh`, not the latest sync, so an open sync PR is no reason to wait. A lesson written only in a report, or in a PR body without the `## bedrock lesson` heading, is never seen by the other repos.

Do not edit the bedrock block of CLAUDE.md, the `bedrock-*` workflows or this skill in this repo: they are overwritten by the next sync. The change goes through the lesson.
