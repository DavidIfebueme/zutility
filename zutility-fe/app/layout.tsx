import type { Metadata } from 'next';
import { Dela_Gothic_One, DM_Sans, JetBrains_Mono } from 'next/font/google';
import { Toaster } from 'sonner';
import './globals.css';

const delaGothicOne = Dela_Gothic_One({
  weight: '400',
  subsets: ['latin'],
  variable: '--font-dela',
});

const dmSans = DM_Sans({
  subsets: ['latin'],
  variable: '--font-dm',
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
});

export const metadata: Metadata = {
  title: 'zutility | ZK Tokens to Naira',
  description: 'Nigerian platform bridging ZK privacy tokens to Naira and utility payments.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html lang="en" className={`${delaGothicOne.variable} ${dmSans.variable} ${jetbrainsMono.variable}`}>
      <body className="font-dm bg-bg-void text-text-primary antialiased" suppressHydrationWarning>
        {children}
        <Toaster theme="dark" position="bottom-right" />
      </body>
    </html>
  );
}
