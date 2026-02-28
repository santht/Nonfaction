'use client';

import { useEffect, useState } from 'react';
import { Download, Search } from 'lucide-react';
import { getStoryPackages, type StoryPackage } from '@/lib/api';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Skeleton } from '@/components/ui/Skeleton';
import { SourceNote } from '@/components/ui/SourceNote';

export default function StoryPackagesPage() {
  const [packages, setPackages] = useState<StoryPackage[]>([]);
  const [query, setQuery] = useState('');
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getStoryPackages().then((data) => {
      setPackages(data);
      setLoading(false);
    });
  }, []);

  async function handleSearch() {
    setLoading(true);
    const data = await getStoryPackages(query);
    setPackages(data);
    setLoading(false);
  }

  return (
    <div className="mx-auto max-w-6xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Story Packages</h1>
      <p className="mt-1 text-sm text-gray-400">Search and export curated source bundles for reporting and research.</p>

      <div className="mt-5 flex gap-2">
        <Input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search by title or entity" />
        <Button onClick={handleSearch}><Search className="h-4 w-4" /> Search</Button>
      </div>

      {loading ? (
        <div className="mt-6 space-y-3"><Skeleton className="h-24" /><Skeleton className="h-24" /></div>
      ) : (
        <div className="mt-6 space-y-3">
          {packages.map((item) => (
            <Card key={item.id}>
              <CardHeader>
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <CardTitle className="text-base">{item.title}</CardTitle>
                    <p className="mt-1 text-sm text-gray-400">{item.summary}</p>
                  </div>
                  <Button size="sm"><Download className="h-3.5 w-3.5" /> ZIP</Button>
                </div>
              </CardHeader>
              <CardContent className="pt-0 text-xs text-gray-400">
                Entities: {item.entities.join(', ')} · Sources: {item.sourceCount} · Date range: {item.dateRange.from} to {item.dateRange.to}
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      <SourceNote text="Source attribution: each package includes a source manifest and citation index for all included records." />
    </div>
  );
}
