# External Blind Review Session

Session id: ext_20260308_161346_e0229d4f
Session token: 88014457314cf0be65eca47a09a74db1
Blind packet: /Users/a/code/RegicideOS/.desloppify/review_packet_blind.json
Template output: /Users/a/code/RegicideOS/.desloppify/external_review_sessions/ext_20260308_161346_e0229d4f/review_result.template.json
Claude launch prompt: /Users/a/code/RegicideOS/.desloppify/external_review_sessions/ext_20260308_161346_e0229d4f/claude_launch_prompt.md
Expected reviewer output: /Users/a/code/RegicideOS/.desloppify/external_review_sessions/ext_20260308_161346_e0229d4f/review_result.json

Happy path:
1. Open the Claude launch prompt file and paste it into a context-isolated subagent task.
2. Reviewer writes JSON output to the expected reviewer output path.
3. Submit with the printed --external-submit command.

Reviewer output requirements:
1. Return JSON with top-level keys: session, assessments, issues.
2. session.id must be `ext_20260308_161346_e0229d4f`.
3. session.token must be `88014457314cf0be65eca47a09a74db1`.
4. Include issues with required schema fields (dimension/identifier/summary/related_files/evidence/suggestion/confidence).
5. Use the blind packet only (no score targets or prior context).
