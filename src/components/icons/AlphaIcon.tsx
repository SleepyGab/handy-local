import React from "react";
import generalMask from "../../assets/nibi-general-mask.png";

const AlphaIcon = ({
  width = 24,
  height = 24,
  className,
}: {
  width?: number | string;
  height?: number | string;
  className?: string;
}) => (
  <span
    aria-hidden="true"
    className={className}
    style={{
      display: "inline-block",
      width,
      height,
      backgroundColor: "currentColor",
      mask: `url(${generalMask}) center / contain no-repeat`,
      WebkitMask: `url(${generalMask}) center / contain no-repeat`,
    }}
  />
);

export default AlphaIcon;
