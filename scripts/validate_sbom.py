#!/usr/bin/env python3
"""Validate a packaged SBOM against hash-pinned official CycloneDX 1.6 schemas."""
import argparse, hashlib, json, tarfile, urllib.request
from jsonschema import Draft7Validator
from referencing import Registry, Resource

SCHEMAS = {
    'bom-1.6.schema.json': '3e92dddbc30cf7f6a02b80f0942b1a4cfd4fb1c26f1dfc4310afa9d613cafb93',
    'spdx.schema.json': 'baa9d3bd1ed57b6751b0887edead6b5063ff53ff7429cf85d476c6c94af0166e',
    'jsf-0.82.schema.json': '8bae002c25e723db7ee1f26afde680ae1a2b1a8f6b4b4b0fd65dc3becb090aae',
}
def validate(archive):
    schemas = {}
    registry = Registry()
    for name, digest in SCHEMAS.items():
        url = 'https://raw.githubusercontent.com/CycloneDX/specification/1.6/schema/' + name
        with urllib.request.urlopen(url, timeout=30) as response:
            data = response.read(1024 * 1024)
        assert hashlib.sha256(data).hexdigest() == digest, 'Official schema changed: ' + name
        schemas[name] = json.loads(data)
        resource = Resource.from_contents(schemas[name])
        for scheme in ['http', 'https']:
            registry = registry.with_resource(scheme + '://cyclonedx.org/schema/' + name, resource)
    with tarfile.open(archive) as package:
        names = [m for m in package.getmembers() if m.name.endswith('/SBOM.cdx.json')]
        assert len(names) == 1 and names[0].size < 8 * 1024 * 1024
        document = json.load(package.extractfile(names[0]))
    Draft7Validator(schemas['bom-1.6.schema.json'], registry=registry).validate(document)
    return {'schema': 'CycloneDX 1.6', 'components': len(document['components']), 'schema_sha256': SCHEMAS, 'passed': True}

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive')
    args = parser.parse_args()
    print(json.dumps(validate(args.archive), indent=2))
