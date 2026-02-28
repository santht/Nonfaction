import { getApiEndpoints } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { SourceNote } from '@/components/ui/SourceNote';

export default async function ApiDocsPage() {
  const endpoints = await getApiEndpoints();

  const jsExample = `const res = await fetch('/api/v1/entities?type=politician');\nconst data = await res.json();`;
  const pyExample = `import requests\nres = requests.get('http://localhost:3000/api/v1/entities')\nprint(res.json())`;
  const curlExample = `curl -H \"Authorization: Bearer <token>\" http://localhost:3000/api/v1/entities`;

  return (
    <div className="mx-auto max-w-6xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">API Documentation</h1>
      <p className="mt-1 text-sm text-gray-400">REST reference for `/api/v1` endpoints.</p>

      <Card className="mt-6 border-yellow-500/30 bg-yellow-500/10">
        <CardContent className="p-4 text-sm text-yellow-100">
          Rate limiting: public endpoints 120 req/min, authenticated writes 30 req/min.
        </CardContent>
      </Card>

      <div className="mt-6 space-y-4">
        {endpoints.map((endpoint) => (
          <Card key={`${endpoint.method}-${endpoint.path}`}>
            <CardHeader>
              <div className="flex items-center gap-2">
                <Badge variant="blue">{endpoint.method}</Badge>
                <CardTitle className="text-base">{endpoint.path}</CardTitle>
                {endpoint.authRequired ? <Badge variant="yellow">Auth required</Badge> : <Badge variant="green">Public</Badge>}
              </div>
            </CardHeader>
            <CardContent className="space-y-2 text-sm text-gray-300">
              <p>{endpoint.description}</p>
              <p className="text-xs text-gray-500">Rate limit: {endpoint.rateLimit}</p>
              {endpoint.requestExample ? <pre className="overflow-x-auto rounded-xl border border-white/10 bg-[#0b1226] p-3 text-xs text-gray-300">{endpoint.requestExample}</pre> : null}
              <pre className="overflow-x-auto rounded-xl border border-white/10 bg-[#0b1226] p-3 text-xs text-gray-300">{endpoint.responseExample}</pre>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="mt-6 grid gap-4 md:grid-cols-3">
        <CodeSample title="curl" code={curlExample} />
        <CodeSample title="JavaScript" code={jsExample} />
        <CodeSample title="Python" code={pyExample} />
      </div>

      <SourceNote text="Source attribution: endpoint examples map to the mock/public API schema used by the platform." />
    </div>
  );
}

function CodeSample({ title, code }: { title: string; code: string }) {
  return (
    <Card>
      <CardHeader><CardTitle className="text-base">{title}</CardTitle></CardHeader>
      <CardContent>
        <pre className="overflow-x-auto rounded-xl border border-white/10 bg-[#0b1226] p-3 text-xs text-gray-300">{code}</pre>
      </CardContent>
    </Card>
  );
}
