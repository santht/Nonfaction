import { getDataSources } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { SourceNote } from '@/components/ui/SourceNote';

export default async function SourcesPage() {
  const sources = await getDataSources();

  const grouped = {
    'Tier 1 Day One': sources.filter((item) => item.tier === 'Tier 1 Day One'),
    'Tier 2 Week One': sources.filter((item) => item.tier === 'Tier 2 Week One'),
    'Tier 3 Month One': sources.filter((item) => item.tier === 'Tier 3 Month One'),
  };

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Data Sources</h1>
      <p className="mt-1 text-sm text-gray-400">108+ source feeds organized by launch tier, data class, and ingest status.</p>

      {Object.entries(grouped).map(([tier, items]) => (
        <Card key={tier} className="mt-6">
          <CardHeader>
            <CardTitle>{tier}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            {items.slice(0, 36).map((item) => (
              <div key={item.id} className="rounded-xl border border-white/10 bg-white/3 p-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <p className="text-sm font-medium text-white">{item.name}</p>
                  <Badge variant={item.status === 'Active' ? 'green' : item.status === 'Planned' ? 'yellow' : 'outline'}>
                    {item.status}
                  </Badge>
                </div>
                <p className="mt-1 text-xs text-gray-400">{item.description}</p>
                <div className="mt-2 flex flex-wrap gap-3 text-xs text-gray-500">
                  <span>Type: {item.dataType}</span>
                  <span>Update: {item.updateFrequency}</span>
                  <a href={item.url} target="_blank" rel="noopener noreferrer" className="text-blue-300 hover:text-blue-200">Source URL</a>
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      ))}

      <SourceNote text="Source attribution: all listed feeds are public-record endpoints or publication archives tracked in ingestion logs." />
    </div>
  );
}
