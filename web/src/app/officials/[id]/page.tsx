import { notFound } from 'next/navigation';
import { Download, Bell, ExternalLink } from 'lucide-react';
import { getOfficialProfile, getRelatedEntities } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Breadcrumb } from '@/components/ui/Breadcrumb';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { SourceNote } from '@/components/ui/SourceNote';

export default async function OfficialProfilePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const profile = await getOfficialProfile(id);

  if (!profile) {
    notFound();
  }

  const related = await getRelatedEntities(profile.relatedEntityIds);

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <Breadcrumb
        items={[
          { label: 'Home', href: '/' },
          { label: 'Officials', href: '/officials' },
          { label: profile.official.name },
        ]}
      />

      <div className="grid gap-6 lg:grid-cols-[1.3fr_0.7fr]">
        <div className="space-y-6">
          <Card>
            <CardHeader>
              <div className="flex flex-wrap items-center gap-2">
                <CardTitle>{profile.official.name}</CardTitle>
                <Badge variant="blue">{profile.official.party}</Badge>
                <Badge variant={profile.official.flagged ? 'red' : 'green'}>{profile.official.flagged ? 'Flagged' : 'Clear'}</Badge>
              </div>
              <p className="text-sm text-gray-400">{profile.official.role} · {profile.official.state} · {profile.official.chamber}</p>
            </CardHeader>
            <CardContent className="text-sm text-gray-300">{profile.bio}</CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">Position history</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              {profile.positionHistory.map((position) => (
                <div key={position.title} className="rounded-xl border border-white/10 bg-white/3 p-3 text-sm text-gray-300">
                  <p className="font-medium text-white">{position.title}</p>
                  <p className="text-xs text-gray-500">{position.startDate} - {position.endDate ?? 'Present'}</p>
                </div>
              ))}
            </CardContent>
          </Card>

          <div className="grid gap-4 md:grid-cols-3">
            <Metric label="Donations received" value={`$${profile.donationsReceived.toLocaleString()}`} />
            <Metric label="PAC contributions" value={`$${profile.pacContributions.toLocaleString()}`} />
            <Metric label="Total funding" value={`$${profile.totalFunding.toLocaleString()}`} />
          </div>

          <Card>
            <CardHeader><CardTitle className="text-base">Voting highlights</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              {profile.votingHighlights.map((item) => (
                <div key={item.bill} className="rounded-xl border border-white/10 bg-white/3 p-3 text-sm">
                  <div className="flex items-center justify-between gap-2">
                    <p className="font-medium text-white">{item.bill}</p>
                    <Badge variant={item.vote === 'Yea' ? 'green' : item.vote === 'Nay' ? 'red' : 'outline'}>{item.vote}</Badge>
                  </div>
                  <p className="mt-1 text-xs text-gray-400">{item.date} · {item.note}</p>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">Timing and conduct snapshot</CardTitle></CardHeader>
            <CardContent className="grid gap-4 md:grid-cols-2">
              <div className="rounded-xl border border-white/10 bg-white/3 p-3 text-sm text-gray-300">
                <p className="mb-1 text-xs text-gray-500">Timing correlations</p>
                <p className="text-2xl font-semibold text-white">{profile.timingCorrelations.length}</p>
              </div>
              <div className="rounded-xl border border-white/10 bg-white/3 p-3 text-sm text-gray-300">
                <p className="mb-1 text-xs text-gray-500">Conduct comparisons</p>
                <p className="text-2xl font-semibold text-white">{profile.conductComparisons.length}</p>
              </div>
              <div className="md:col-span-2 flex h-56 items-center justify-center rounded-xl border border-dashed border-white/20 bg-white/3 text-sm text-gray-400">
                Connections graph placeholder
              </div>
            </CardContent>
          </Card>
        </div>

        <aside className="space-y-6">
          <Card>
            <CardHeader><CardTitle className="text-base">Actions</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              <Button className="w-full"><Download className="h-4 w-4" /> Export story package</Button>
              <Button variant="secondary" className="w-full"><Bell className="h-4 w-4" /> Subscribe to watchlist</Button>
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">Related entities</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              {related.map((item) => (
                <div key={item.id} className="rounded-xl border border-white/10 bg-white/3 p-3">
                  <p className="text-sm text-white">{item.name}</p>
                  <p className="text-xs text-gray-500">{item.type} · {item.connectionCount} connections</p>
                </div>
              ))}
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">Source links</CardTitle></CardHeader>
            <CardContent>
              <a href="https://www.congress.gov" target="_blank" rel="noopener noreferrer" className="inline-flex items-center gap-1 text-sm text-blue-300 hover:text-blue-200">
                <ExternalLink className="h-3.5 w-3.5" /> Congress.gov reference
              </a>
            </CardContent>
          </Card>
        </aside>
      </div>

      <SourceNote text="Source attribution: official profile data references campaign finance, vote records, and disclosure datasets." />
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <Card>
      <CardContent className="p-4">
        <p className="text-xs text-gray-500">{label}</p>
        <p className="mt-2 text-xl font-semibold text-white">{value}</p>
      </CardContent>
    </Card>
  );
}
