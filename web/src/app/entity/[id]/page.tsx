import EntityClient from './EntityClient';

export function generateStaticParams() {
  return [
    { id: 'e1' }, { id: 'e2' }, { id: 'e3' }, { id: 'e4' },
    { id: 'e5' }, { id: 'e6' }, { id: 'e7' }, { id: 'e8' },
  ];
}

export default async function EntityPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <EntityClient id={id} />;
}
