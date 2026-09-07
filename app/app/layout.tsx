import type { Metadata } from "next";
import "@solana/wallet-adapter-react-ui/styles.css";
import "./styles.css";
import { Providers } from "@/components/providers";

export const metadata: Metadata = { title: "Aegis Markets", description: "Aave V3-inspired lending on Solana devnet" };
export default function Layout({ children }: Readonly<{ children: React.ReactNode }>) {
  return <html lang="zh-CN"><body><Providers>{children}</Providers></body></html>;
}

