import { useId } from 'react';

export function BookCoverOverlay({ className }: { className?: string }) {
  const uid = useId().replace(/:/g, '');
  const clipId = `q-book-cover-wash-clip-${uid}`;
  const paintId = `q-book-cover-wash-radial-${uid}`;

  return (
    <svg
      className={className}
      viewBox="0 0 130 166"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      preserveAspectRatio="none"
      aria-hidden="true"
    >
      <g style={{ mixBlendMode: 'overlay' }} opacity="0.5" clipPath={`url(#${clipId})`}>
        <rect width="130" height="166" fill="white" />
        <rect width="130" height="166" fill={`url(#${paintId})`} />
      </g>
      <defs>
        <radialGradient
          id={paintId}
          cx="0"
          cy="0"
          r="1"
          gradientUnits="userSpaceOnUse"
          gradientTransform="translate(65) rotate(90) scale(166 178.164)"
        >
          <stop offset="0.0520833" stopColor="#00FF84" stopOpacity="0.33" />
          <stop offset="0.447931" stopColor="#0075FF" stopOpacity="0.38" />
          <stop offset="1" stopColor="white" stopOpacity="0" />
        </radialGradient>
        <clipPath id={clipId}>
          <rect width="130" height="166" fill="white" />
        </clipPath>
      </defs>
    </svg>
  );
}
