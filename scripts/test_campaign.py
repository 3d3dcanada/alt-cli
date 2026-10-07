#!/usr/bin/env python3
"""Exercise actual frozen evaluator execution; no weights or quality claim."""
import subprocess,sys,tempfile,unittest
from pathlib import Path
from campaign_v5 import freeze_evaluators,sha

class CampaignTests(unittest.TestCase):
    def test_running_campaign_does_not_import_new_checkout_oracles(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);source=root/'checkout';source.mkdir();output=root/'campaign';output.mkdir()
            (source/'oracle.py').write_text("def check():return 'original oracle'\n")
            (source/'runner.py').write_text('from oracle import check\nprint(check())\n')
            inputs={p.name:sha(p) for p in source.iterdir()};frozen=freeze_evaluators(output,inputs,False,source)
            (source/'oracle.py').write_text("def check():return 'changed answer'\n")
            freeze_evaluators(output,inputs,True,source)
            executed=subprocess.check_output([sys.executable,str(frozen/'runner.py')],text=True)
            self.assertEqual(executed.strip(),'original oracle')
            (frozen/'oracle.py').write_text("def check():return 'tampered'\n")
            with self.assertRaises(AssertionError):freeze_evaluators(output,inputs,True,source)
    def test_missing_frozen_source_cannot_be_reconstructed_on_resume(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);source=root/'checkout';source.mkdir();output=root/'campaign';output.mkdir();file=source/'oracle.py';file.write_text('# original\n')
            inputs={file.name:sha(file)};frozen=freeze_evaluators(output,inputs,False,source);(frozen/file.name).unlink()
            with self.assertRaises(AssertionError):freeze_evaluators(output,inputs,True,source)

if __name__=='__main__':unittest.main()
