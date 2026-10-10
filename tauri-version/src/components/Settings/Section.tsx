import type { ReactNode } from 'react';
import './Section.css';

interface SectionProps {
  title: string;
  children: ReactNode;
}

export function Section({ title, children }: SectionProps) {
  return (
    <section className="q-settings-block">
      <h3 className="q-settings-block__header">{title}</h3>
      <div className="q-settings-block__card">{children}</div>
    </section>
  );
}
