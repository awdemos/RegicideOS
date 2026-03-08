# External Blind Review Session

Session id: ext_20260301_214811_38b5a9db
Session token: f884be58399098577150dbd1fc38c097
Blind packet: /Users/a/code/RegicideOS/.desloppify/review_packet_blind.json
Template output: /Users/a/code/RegicideOS/.desloppify/external_review_sessions/ext_20260301_214811_38b5a9db/review_result.template.json
Claude launch prompt: /Users/a/code/RegicideOS/.desloppify/external_review_sessions/ext_20260301_214811_38b5a9db/claude_launch_prompt.md
Expected reviewer output: /Users/a/code/RegicideOS/.desloppify/external_review_sessions/ext_20260301_214811_38b5a9db/review_result.json

Happy path:
1. Open the Claude launch prompt file and paste it into a context-isolated subagent task.
2. Reviewer writes JSON output to the expected reviewer output path.
3. Submit with the printed --external-submit command.

Reviewer output requirements:
1. Return JSON with top-level keys: session, assessments, findings.
2. session.id must be `ext_20260301_214811_38b5a9db`.
3. session.token must be `f884be58399098577150dbd1fc38c097`.
4. Include findings with required schema fields (dimension/identifier/summary/related_files/evidence/suggestion/confidence).
5. Use the blind packet only (no score targets or prior context).
