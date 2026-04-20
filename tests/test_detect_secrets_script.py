import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[1]
SCRIPT_PATH = REPO_ROOT / "scripts" / "check-detect-secrets.sh"


class DetectSecretsScriptTests(unittest.TestCase):
    def run_scan(self, content: str) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as tmp:
            repo = Path(tmp)
            script = repo / "scripts" / "check-detect-secrets.sh"
            script.parent.mkdir()
            shutil.copy2(SCRIPT_PATH, script)

            subprocess.run(["git", "init", "-q"], cwd=repo, check=True)
            candidate = repo / "candidate.txt"
            candidate.write_text(content, encoding="utf-8")
            subprocess.run(["git", "add", "candidate.txt"], cwd=repo, check=True)

            return subprocess.run(
                ["bash", str(script), "--staged"],
                cwd=repo,
                check=False,
                text=True,
                capture_output=True,
            )

    def assert_detected(self, content: str) -> None:
        result = self.run_scan(content)
        self.assertNotEqual(
            result.returncode,
            0,
            msg=f"expected secret detection to fail; stderr={result.stderr!r}",
        )

    def test_detects_line_anchored_anthropic_api_key_assignment(self) -> None:
        key_name = "ANTHROPIC" + "_API_KEY"
        self.assert_detected(f"{key_name}=sk-ant-fake123\n")

    def test_detects_aws_session_token(self) -> None:
        token = "ASIA" + "IOSFODNN7EXAMPLE"
        self.assert_detected(f"aws_session_token = {token}\n")

    def test_detects_bearer_token(self) -> None:
        token = "Bearer " + "eyJhbGciOiJIUzI1NiJ9.fake"
        self.assert_detected(f"Authorization: {token}\n")

    def test_detects_dsa_private_key_header(self) -> None:
        header = "BEGIN " + "DSA PRIVATE KEY"
        self.assert_detected(f"-----{header}-----\n")

    def test_detects_generic_password_assignment(self) -> None:
        name = "pass" + "word"
        self.assert_detected(f"{name} = 'not-a-real-secret'\n")

    def test_allows_documented_anthropic_variable_name(self) -> None:
        key_name = "ANTHROPIC" + "_API_KEY"
        result = self.run_scan(f"# set {key_name}= in your shell\n")
        self.assertEqual(0, result.returncode, msg=result.stderr)


if __name__ == "__main__":
    unittest.main()
