import { BlinkSection } from "@/components/Sections/blinksSection"
import { FooterSection } from "@/components/Sections/footerSection"
import { HeroSection } from "@/components/Sections/heroSection"
import { PrinciplesSection } from "@/components/Sections/principlesSection"
import { SignalsSection } from "@/components/Sections/signalSection"
import { SideNav } from "@/components/side-nav"

export default function Page() {
  return (
    <main className="relative min-h-screen">
      <SideNav />
      <div className="grid-bg fixed inset-0 opacity-30" aria-hidden="true" />

      <div className="relative z-10">
        <HeroSection/>
        <SignalsSection/>
        <BlinkSection/>
        <PrinciplesSection/>
        <FooterSection/>

      </div>
    </main>
  )
}
