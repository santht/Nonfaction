import { getPlatformUpdates } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent } from '@/components/ui/Card';
import { SourceNote } from '@/components/ui/SourceNote';

export default async function UpdatesPage() {
  const updates = await getPlatformUpdates();

  return (
    <div className="mx-auto max-w-5xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Updates</h1>
      <p className="mt-1 text-sm text-gray-400">Release notes, milestones, and source expansion announcements.</p>

      <div className="mt-6 space-y-3">
        {updates.map((item) => (
          <Card key={item.id}>
            <CardContent className="p-4">
              <div className="mb-2 flex items-center gap-2">
                <Badge variant={item.category === 'release' ? 'blue' : item.category === 'sources' ? 'green' : 'yellow'}>
                  {item.category}
                </Badge>
                <span className="text-xs text-gray-500">{item.date}</span>
              </div>
              <p className="text-sm font-medium text-white">{item.title}</p>
              <p className="mt-1 text-sm text-gray-400">{item.summary}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <SourceNote text="Source attribution: update entries reflect release logs and ingestion pipeline milestones." />
    </div>
  );
}
