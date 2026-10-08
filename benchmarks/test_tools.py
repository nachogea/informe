import json
from pathlib import Path
import tempfile
import unittest
from summarize import read_trials, summarize
from build_fixtures import render, text

class BenchmarkTools(unittest.TestCase):
    def record(self,**kwargs):
        return dict(task='test',format='native_json',model='fixed',environment='test-machine',scenario='generation',success=True,**kwargs)
    def test_failures_retain_usage_without_polluting_success_latency(self):
        good=self.record(input_tokens=10,output_tokens=4,generation_ms=20)
        bad={**good,'success':False,'input_tokens':30,'generation_ms':999}
        report=summarize([good,bad])[0]
        self.assertEqual(report['success_rate'],.5)
        self.assertEqual(report['usage_all_trials']['input_tokens']['total'],40)
        self.assertEqual(report['metrics_successful_trials']['generation_ms']['median'],20)
    def test_rejects_bad_usage_and_numeric_values(self):
        with tempfile.TemporaryDirectory() as d:
            p=Path(d)/'results.jsonl'
            for field,value in [('input_tokens',True),('open_ms',-1),('output_tokens',1.5)]:
                p.write_text(json.dumps(self.record(**{field:value})))
                with self.assertRaises(ValueError):read_trials(p)
            p.write_text(json.dumps(self.record(input_tokens=1,cached_input_tokens=2)))
            with self.assertRaises(ValueError):read_trials(p)
    def test_html_fixture_escapes_content_and_copies_rectangular_table(self):
        n={'type':'table','id':'quotes"','title':'<script>alert(1)</script>','columns':['A','B'],'rows':[['line\nbreak','tab\tcell']]}
        rendered=render(n)
        self.assertNotIn('<script>',rendered)
        self.assertIn('&lt;script&gt;',rendered)
        self.assertEqual(text(n),'A\tB\nline break\ttab cell')
if __name__=='__main__':unittest.main()
