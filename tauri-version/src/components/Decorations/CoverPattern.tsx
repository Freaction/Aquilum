import { coverPatternClassName, type CoverPatternId } from '../../modules/docs/coverPatterns';
import { CityCoverPattern } from './CityCoverPattern';
import './coverPatterns/base.css';
import './coverPatterns/stripes.css';
import './coverPatterns/hearts.css';
import './coverPatterns/cubes.css';
import './coverPatterns/origami.css';
import './coverPatterns/foliage.css';
import './coverPatterns/arrows.css';
import './coverPatterns/moire.css';
import './coverPatterns/parquet.css';
import './coverPatterns/ripples.css';
import './coverPatterns/trellis.css';
import './coverPatterns/grids.css';
import './coverPatterns/radial.css';
import './coverPatterns/dreams.css';
import './coverPatterns/polka.css';
import './coverPatterns/weave.css';
import './coverPatterns/quadrants.css';
import './coverPatterns/woven-tiles.css';
import './coverPatterns/city.css';
import './coverPatterns/tunnel.css';

const DREAM_PARTICLES = Array.from({ length: 40 }, (_, index) => ({
  left: `${(index * 37) % 100}%`,
  size: `${4 + ((index * 13) % 7)}px`,
  delay: `${-((index * 1700) % 28000)}ms`,
  duration: `${24000 + ((index * 2300) % 9000)}ms`,
}));

export function CoverPattern({ id }: { id: CoverPatternId }) {
  return (
    <div className={coverPatternClassName(id)} aria-hidden="true">
      {id === 'dreams' && DREAM_PARTICLES.map((particle, index) => (
        <span
          className="q-cover-pattern__particle"
          key={index}
          style={{
            left: particle.left,
            width: particle.size,
            height: particle.size,
            animationDelay: particle.delay,
            animationDuration: particle.duration,
          }}
        />
      ))}
      {id === 'city' && <CityCoverPattern />}
    </div>
  );
}
