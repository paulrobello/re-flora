import React, { useState } from 'react';
import { Leaf, Sun, Wind, Heart, Sprout, Layers } from 'lucide-react';
import Seal from '../components/Seal';

export default function Home() {
  const [isSealed, setIsSealed] = useState(true);

  return (
    <>
      {isSealed && <Seal onOpen={() => setIsSealed(false)} />}
      
      <div className="min-h-screen py-8 md:py-16 px-4 flex justify-center items-start">
        {/* Letter Container */}
        <div className="letter-paper max-w-5xl w-full mx-auto rounded-sm md:rounded-md p-8 md:p-16 text-slate-800 shadow-2xl transition-all duration-1000 ease-out transform"
             style={{ 
               opacity: isSealed ? 0 : 1, 
               transform: isSealed ? 'translateY(50px) scale(0.95)' : 'translateY(0) scale(1)' 
             }}>
          
          {/* Sparkling Lines (Decoration) */}
          <div className="absolute top-20 left-0 w-full h-px opacity-30 pointer-events-none">
             <div className="sparkle-line" style={{ animationDelay: '0s', top: '10%' }}></div>
             <div className="sparkle-line" style={{ animationDelay: '2s', top: '40%' }}></div>
             <div className="sparkle-line" style={{ animationDelay: '1s', top: '70%' }}></div>
          </div>

          {/* Header / Letterhead */}
          <header className="relative flex flex-col items-center justify-center mb-16 text-center border-b-2 border-slate-200 pb-12">
            <div className="max-w-3xl w-full mb-8 transform hover:scale-[1.01] transition-transform duration-500">
              <img
                src="/title.png"
                alt="Re: Flora Title"
                className="w-full h-auto rounded-lg shadow-lg sepia-[0.2]"
              />
            </div>
            <h1 className="text-5xl md:text-7xl font-bold text-slate-800 mb-6 tracking-tight font-serif">
              Re: Flora
            </h1>
            <p className="text-xl md:text-2xl text-slate-600 max-w-2xl mx-auto italic font-serif leading-relaxed">
              "An experimental relaxation game where you design and nurture your own island paradise."
            </p>
          </header>

          {/* Main Content */}
          <main className="space-y-16">
            
            {/* Overview Section */}
            <section className="text-center max-w-3xl mx-auto">
              <h2 className="text-3xl font-bold text-slate-800 mb-6 font-serif border-b border-slate-300 inline-block pb-2">A Digital Sanctuary</h2>
              <p className="text-lg text-slate-700 leading-loose font-serif">
                Using vibrant voxel rendering, <strong>Re: Flora</strong> allows you to cultivate a diverse ecosystem of plants,
                shape terrain, and create a personal sanctuary. The game emphasizes creativity and tranquility
                with no failure states, focusing instead on the joy of watching your garden evolve.
              </p>
            </section>

            {/* Features Grid */}
            <section>
              <h2 className="text-3xl font-bold text-center text-slate-800 mb-12 font-serif">Features</h2>
              <div className="grid md:grid-cols-2 gap-8">
                <FeatureCard
                  icon={<Sprout className="w-6 h-6 text-emerald-700" />}
                  title="Intuitive Planting"
                  description="Easily select, place, and nurture various plant species in your garden."
                />
                <FeatureCard
                  icon={<Wind className="w-6 h-6 text-sky-700" />}
                  title="Dynamic Ecosystem"
                  description="Watch plants grow, spread, and interact based on environmental conditions."
                />
                <FeatureCard
                  icon={<Sun className="w-6 h-6 text-amber-600" />}
                  title="Day & Night Cycles"
                  description="Experience visual changes and different growth patterns as time passes."
                />
                <FeatureCard
                  icon={<Heart className="w-6 h-6 text-rose-600" />}
                  title="Relaxing Atmosphere"
                  description="Meditative audio, gentle animations, and a stress-free experience."
                />
              </div>
            </section>

            {/* Botanical Reality */}
            <section className="bg-slate-50 p-8 rounded-lg border border-slate-200">
              <div className="flex flex-col md:flex-row items-start gap-12">
                <div className="flex-1">
                  <h2 className="text-2xl font-bold mb-6 font-serif text-slate-800">Botanical Reality</h2>
                  <ul className="space-y-4">
                    <li className="flex items-center gap-3 text-slate-700">
                      <Leaf className="w-5 h-5 text-emerald-600" />
                      <span>Realistic growth cycles</span>
                    </li>
                    <li className="flex items-center gap-3 text-slate-700">
                      <Leaf className="w-5 h-5 text-emerald-600" />
                      <span>Environmental preferences</span>
                    </li>
                    <li className="flex items-center gap-3 text-slate-700">
                      <Leaf className="w-5 h-5 text-emerald-600" />
                      <span>Seasonal behaviors</span>
                    </li>
                    <li className="flex items-center gap-3 text-slate-700">
                      <Leaf className="w-5 h-5 text-emerald-600" />
                      <span>Educational elements</span>
                    </li>
                  </ul>
                </div>
                <div className="flex-1">
                   <div className="bg-emerald-900/5 p-6 rounded-lg border-l-4 border-emerald-700 italic text-slate-700 font-serif">
                      We're integrating elements of real-world botany to create a world that feels alive and grounded, yet magical.
                   </div>
                   <div className="mt-8 text-center">
                    <a
                      href="https://github.com/re-flora/re-flora"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="inline-flex items-center gap-2 px-6 py-3 bg-slate-800 text-white rounded hover:bg-slate-700 transition-colors font-serif tracking-wide"
                    >
                      <Layers className="w-4 h-4" />
                      Visit Repository
                    </a>
                   </div>
                </div>
              </div>
            </section>

          </main>

          {/* Footer */}
          <footer className="mt-20 pt-10 border-t border-slate-200 text-center text-slate-500 font-serif italic">
            <p className="mb-2">Re: Flora — An experimental project.</p>
            <p className="text-sm">Built with Rust, Vulkan, and a love for nature.</p>
          </footer>
        </div>
      </div>
    </>
  );
}

function FeatureCard({ icon, title, description }: { icon: React.ReactNode, title: string, description: string }) {
  return (
    <div className="flex gap-4 p-4 hover:bg-slate-50 rounded-lg transition-colors group">
      <div className="shrink-0 mt-1 p-2 bg-white border border-slate-200 rounded shadow-sm group-hover:shadow-md transition-shadow h-fit">
        {icon}
      </div>
      <div>
        <h3 className="text-lg font-bold text-slate-800 mb-1 font-serif">{title}</h3>
        <p className="text-slate-600 text-sm leading-relaxed">{description}</p>
      </div>
    </div>
  );
}
