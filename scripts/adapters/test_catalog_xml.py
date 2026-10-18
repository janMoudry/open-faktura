import importlib.util
import json
from pathlib import Path
import unittest
spec=importlib.util.spec_from_file_location("catalog_xml",Path(__file__).with_name("catalog_xml.py"))
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
ROOT=Path(__file__).resolve().parents[2]
class Contract(unittest.TestCase):
 def setUp(self):
  self.raw=(ROOT/"tests/fixtures/adapters/synthetic-user-catalog.xml").read_bytes()
  self.mapping=json.loads((ROOT/"tests/fixtures/adapters/synthetic-user-catalog.mapping.json").read_text())
 def run_adapter(self,raw=None):return module.convert(raw or self.raw,self.mapping,"user-defined-test","00000000-0000-4000-8000-000000000001")
 def test_exact_prices_keys_and_no_inferred_currency(self):
  result=self.run_adapter();self.assertEqual(result["source"],"generic");self.assertEqual(result["records"][0]["key"],"00001");self.assertEqual(result["records"][0]["data"]["unit_price"],"19.995");self.assertEqual(result["records"][1]["data"]["unit_price"],"0");self.assertEqual(result["records"][0]["data"]["code"],"00005")
 def test_fail_closed_inputs(self):
  for raw in [self.raw.replace(b"19,995",b""),self.raw.replace(b"19,995",b"1.000,00"),self.raw.replace(b"19,995",b"1e2"),self.raw.replace(b'00002',b'00001'),b'<!DOCTYPE x>'+self.raw]:
   with self.subTest(raw=raw),self.assertRaises((ValueError,module.ET.ParseError)):self.run_adapter(raw)
 def test_explicit_mapping_and_unknown_paths(self):
  self.mapping["fields"]["currency"]={"constant":"EUR"}
  with self.assertRaises(ValueError):self.run_adapter()
  self.mapping["fields"]["currency"]={"constant":"CZK"};self.mapping["record_path"]="Unknown/Item"
  with self.assertRaises(ValueError):self.run_adapter()
if __name__=="__main__":unittest.main()
