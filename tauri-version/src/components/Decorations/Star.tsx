import { useId } from 'react';
import './Star.css';

export function Star() {
  const uid = useId().replace(/[^a-zA-Z0-9_-]/g, '');
  const path = "M26.7326 32.142C34.8108 26.881 36.851 0 36.851 0C36.851 0 38.6803 24.7488 46 30C54.5 36.0979 96 38.5621 96 38.5621C96 38.5621 53.7749 39.0195 45.6578 44.4024C37.5407 49.7852 36.851 63 36.851 63C36.851 63 35.1929 49.7069 26.7326 44.4024C18.7456 39.3946 0 38.5621 0 38.5621C0 38.5621 18.8107 37.3012 26.7326 32.142Z";
  const mask = `url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 96 63'%3E%3Cpath d='${path}' fill='black'/%3E%3C/svg%3E")`;

  return (
    <div
      className="q-book-star"
      style={{ maskImage: mask, WebkitMaskImage: mask }}
    >
      <svg
        className="q-book-star__svg"
        viewBox="0 0 96 63"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <g filter={`url(#${uid}filter0_ii_148_2537)`}>
          <path d={path} fill="white" fillOpacity="0.4" />
        </g>
        <defs>
          <filter id={`${uid}filter0_ii_148_2537`} x="-36.5556" y="-36.5556" width="169.111" height="136.111" filterUnits="userSpaceOnUse" colorInterpolationFilters="sRGB">
            <feFlood floodOpacity="0" result="BackgroundImageFix" />
            <feBlend mode="normal" in="SourceGraphic" in2="BackgroundImageFix" result="shape" />
            <feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha" />
            <feOffset dx="-1.93416" dy="-1.93416" />
            <feGaussianBlur stdDeviation="1.93416" />
            <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
            <feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.5 0" />
            <feBlend mode="normal" in2="shape" result="effect1_innerShadow_148_2537" />
            <feColorMatrix in="SourceAlpha" type="matrix" values="0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" result="hardAlpha" />
            <feOffset dx="1.93416" dy="11" />
            <feGaussianBlur stdDeviation="3.86832" />
            <feComposite in2="hardAlpha" operator="arithmetic" k2="-1" k3="1" />
            <feColorMatrix type="matrix" values="0 0 0 0 1 0 0 0 0 1 0 0 0 0 1 0 0 0 0.3 0" />
            <feBlend mode="normal" in2="effect1_innerShadow_148_2537" result="effect2_innerShadow_148_2537" />
          </filter>
        </defs>
      </svg>
    </div>
  );
}
