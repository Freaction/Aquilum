const el = (className: string, children?: React.ReactNode, data?: Record<string, string>) => (
  <div className={className} {...(data ? Object.fromEntries(Object.entries(data).map(([key, value]) => [`data-${key}`, value])) : {})}>{children}</div>
);

function Tower({ name, children }: { name: string; children: React.ReactNode }) {
  return el(`tower tower${name}`, children);
}

function CityScene() {
  return (
    <div className="area">
      {el('field')}{el('load', el('line'))}
      {el('tree tree01')}{el('tree tree02', el('leaf'))}{el('tree tree03', el('leaf'))}{el('tree tree02 pos02', el('leaf'))}{el('tree tree03 pos02', el('leaf'))}
      {el('hydrant pos01', el('line'))}{el('hydrant pos02', el('line'))}
      {el('back_building building01')}{el('back_building building02')}{el('back_building building03')}{el('back_building building04')}
      {el('sign', <>{el('panel pos01')}{el('panel pos02')}{el('panel pos03')}</>)}
      {el('traffic_light', <>{el('circle red')}{el('circle yellow')}{el('circle green')}</>)}
      {el('street_lamp street_lamp01', <>{el('light left')}{el('light right')}</>)}{el('street_lamp street_lamp02', el('light'))}
      {el('cloud cloud01', <>{el('circle circle01')}{el('circle circle02')}</>)}
      {el('cloud cloud02', <>{el('circle circle01')}{el('circle circle02')}{el('circle circle03')}</>)}{el('cloud cloud03', el('circle circle01'))}
      <Tower name="01">{el('chimney chimney01')}{['0','1','2','0','3','4','0','0'].map((h, i) => el('window window01', undefined, { h, pos: String(i) }))}{el('door door01')}{el('stair', <>{el('side pos01', el('deck'))}{el('side pos02', el('deck'))}</>)}</Tower>
      <Tower name="02">{el('chimney chimney02')}{['1','2','0','3','4','0','2','0'].map((h, i) => el('window window01', undefined, { h, pos: String(i) }))}{el('door door02', el('deck'))}</Tower>
      <Tower name="03">{el('floor', <>{el('chimney chimney01')}{el('window window02', undefined, { h: '0', pos: '0' })}{el('window window02', undefined, { h: '1', pos: '1' })}</>)}{el('window window03', el('deck'))}{el('door door03', el('deck'))}</Tower>
      <Tower name="04">{el('billboard', el('deck'))}{el('kiosk', <>{el('deck01')}{el('deck02')}{el('deck03')}{el('deck04')}</>)}{el('door door01')}</Tower>
      <Tower name="05">{el('chimney chimney01')}{el('window window01', undefined, { h: '5', pos: '0' })}{el('window window01', undefined, { h: '0', pos: '1' })}{el('window window01', undefined, { h: '6', pos: '2' })}{el('window window04', undefined, { s: '0', pos: '3' })}{el('window window04', undefined, { s: '1', pos: '4' })}{el('kiosk', <>{el('deck01')}{el('deck02')}{el('deck03')}{el('deck04')}</>)}{el('door door01')}</Tower>
      {el('balloon balloon01', el('deck'))}{el('balloon balloon02', el('deck'))}
    </div>
  );
}

export function CityCoverPattern() {
  return <div className="q-city-pattern__strip">{Array.from({ length: 9 }, (_, index) => <div className="q-city-pattern__scene bg01" key={index}><CityScene /></div>)}</div>;
}
