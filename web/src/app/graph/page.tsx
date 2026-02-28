import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Select } from '@/components/ui/Select';
import { SourceNote } from '@/components/ui/SourceNote';

export default function GraphPage() {
  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <h1 className="text-3xl font-semibold text-white">Network Graph Explorer</h1>
      <p className="mt-1 text-sm text-gray-400">Interactive relationship map placeholder for upcoming Cytoscape.js integration.</p>

      <div className="mt-6 grid gap-6 lg:grid-cols-[1fr_320px]">
        <Card>
          <CardHeader><CardTitle>Graph canvas</CardTitle></CardHeader>
          <CardContent>
            <div className="flex h-[520px] items-center justify-center rounded-xl border border-dashed border-white/20 bg-gradient-to-br from-blue-500/10 to-transparent text-sm text-gray-400">
              Cytoscape visualization placeholder
            </div>
          </CardContent>
        </Card>

        <div className="space-y-4">
          <Card>
            <CardHeader><CardTitle className="text-base">Controls</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              <Select defaultValue="all"><option className="bg-[#0a0f1c]" value="all">All entity types</option><option className="bg-[#0a0f1c]" value="official">Officials</option><option className="bg-[#0a0f1c]" value="corp">Corporations</option></Select>
              <Select defaultValue="all"><option className="bg-[#0a0f1c]" value="all">All relationships</option><option className="bg-[#0a0f1c]" value="financial">Financial</option><option className="bg-[#0a0f1c]" value="meeting">Meeting</option></Select>
              <Select defaultValue="2024"><option className="bg-[#0a0f1c]" value="2024">2024</option><option className="bg-[#0a0f1c]" value="2025">2025</option></Select>
            </CardContent>
          </Card>
          <Card>
            <CardHeader><CardTitle className="text-base">Selected node</CardTitle></CardHeader>
            <CardContent className="text-sm text-gray-400">
              Click any node in the graph to inspect details, source links, and connection metadata.
            </CardContent>
          </Card>
        </div>
      </div>

      <SourceNote text="Source attribution: graph edges map to verified record references from source-linked connection data." />
    </div>
  );
}
