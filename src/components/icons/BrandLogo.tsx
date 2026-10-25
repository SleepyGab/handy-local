import React from "react";
import { useTranslation } from "react-i18next";
import avatar from "../../assets/nibi-avatar.png";

const BrandLogo = ({
  width = 160,
  height,
  className,
}: {
  width?: number;
  height?: number;
  className?: string;
}) => {
  const { t } = useTranslation();
  const logoHeight = height ?? width / 3;
  return (
    <div
      className={className}
      style={{
        width,
        height: logoHeight,
        display: "flex",
        alignItems: "center",
        gap: logoHeight / 8,
      }}
    >
      <img
        src={avatar}
        alt=""
        style={{
          height: logoHeight,
          width: "auto",
          objectFit: "contain",
          flexShrink: 0,
        }}
      />
      <span
        style={{
          fontSize: logoHeight * 0.8,
          fontWeight: 750,
          lineHeight: 1,
          letterSpacing: "-0.04em",
        }}
      >
        {t("app.name")}
      </span>
    </div>
  );
};

export default BrandLogo;
