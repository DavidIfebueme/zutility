import type { Metadata } from 'next';
import { Fraunces, Geist, Geist_Mono } from 'next/font/google';
import { Toaster } from 'sonner';
import './globals.css';

const fraunces = Fraunces({
  subsets: ['latin'],
  variable: '--font-fraunces',
  display: 'swap',
});

const geist = Geist({
  subsets: ['latin'],
  variable: '--font-geist',
  display: 'swap',
});

const geistMono = Geist_Mono({
  subsets: ['latin'],
  variable: '--font-geist-mono',
  display: 'swap',
});

export const metadata: Metadata = {
  title: 'zutility | Pay utilities with Zcash',
  description:
    'Pay electricity, airtime, data, and TV with ZEC. No KYC. Pay from any Zcash wallet.',
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html
      lang="en"
      className={`${fraunces.variable} ${geist.variable} ${geistMono.variable}`}
    >
      <body className="font-sans bg-paper text-ink antialiased" suppressHydrationWarning>
        {children}
        <Toaster
          theme="light"
          position="bottom-right"
          toastOptions={{
            style: {
              background: '#fffdf7',
              border: '1px solid #d3c9b8',
              color: '#14130f',
              borderRadius: '8px',
            },
          }}
        />
      </body>
    </html>
  );
}
