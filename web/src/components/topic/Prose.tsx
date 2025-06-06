// import 'prismjs/themes/prism.css';
import '../../styles/code.css';
import 'yet-another-react-lightbox/styles.css';

import * as Prism from 'prismjs';
import Lightbox from 'yet-another-react-lightbox';
import Zoom from 'yet-another-react-lightbox/plugins/zoom';

// This ensures prism is loaded first
// @ts-ignore
// eslint-disable-next-line
const data = Prism.util;

import 'prismjs/components/prism-solidity';
import 'prismjs/components/prism-go';

import { FC, useEffect, useRef, useState } from 'react';

const trackLinkClick = async (url: string, topicId: number, postId: number) => {
    const formData = new FormData();

    formData.append('url', url);
    formData.append('post_id', postId.toString());
    formData.append('topic_id', topicId.toString());

    await fetch('https://ethereum-magicians.org/clicks/track', {
        body: formData,
        method: 'POST',
        mode: 'no-cors',
        credentials: 'omit',
    });
};

export function ImageLightbox() {
    const [open, setOpen] = useState(false);
    const [src, setSrc] = useState<string | null>(null);

    useEffect(() => {
        const handler = (e: Event) => {
            const customEvent = e as CustomEvent<{ src: string }>;

            if (!customEvent.detail || !customEvent.detail.src) {
                return;
            }

            setSrc(customEvent.detail.src);
            setOpen(true);
        };

        window.addEventListener('open-image-lightbox', handler);

        return () => window.removeEventListener('open-image-lightbox', handler);
    }, []);

    return (
        <Lightbox
            open={open}
            close={() => setOpen(false)}
            slides={src ? [{ src }] : []}
            plugins={[Zoom]}
            zoom={{ maxZoomPixelRatio: 8 }}
            render={{
                buttonPrev: () => null,
                buttonNext: () => null,
            }}
        />
    );
}

export const Prose: FC<{ content: string; topicId: number; postId: number }> = ({
    content,
    topicId,
    postId,
}) => {
    const ref = useRef<HTMLDivElement>(null);

    useEffect(() => {
        const container = ref.current;

        if (!container) return;

        const anchors = Array.from(container.querySelectorAll('a'));
        // store handlers so we can clean up
        const handlers = anchors.map((a) => {
            const img = a.querySelector('img');
            let onClick: (e: MouseEvent) => void;

            if (
                img &&
                a.href.startsWith('https://ethereum-magicians.org/uploads/default/original/')
            ) {
                onClick = (e: MouseEvent) => {
                    e.preventDefault();
                    e.stopPropagation();
                    window.dispatchEvent(
                        new CustomEvent('open-image-lightbox', { detail: { src: a.href } })
                    );
                };
            } else {
                onClick = (e: MouseEvent) => {
                    e.preventDefault();
                    e.stopPropagation();

                    (async () => {
                        await trackLinkClick(a.href, topicId, postId);
                        window.open(a.href, '_blank', 'noopener,noreferrer');
                    })();
                };
            }

            a.setAttribute('target', '_blank');
            a.setAttribute('rel', 'noopener noreferrer');
            a.addEventListener('click', onClick);

            return { a, onClick };
        });

        const code = container.querySelectorAll('code');

        for (const c of code) {
            const langClass = [...c.classList.values()].find((l) => l.startsWith('language-'));
            const lang = langClass?.replace('language-', '');

            // console.log(lang);

            if (lang == 'auto') {
                // guess the lang and update it
                const content = c.textContent;

                // console.log('detecting lang ', content);

                if (['contract', 'uint256', 'address'].some((w) => content?.includes(w))) {
                    // console.log('detected solidity');
                    c.classList.add('language-solidity');
                    c.classList.remove('language-auto');
                }
            }

            if (lang == 'sol') {
                c.classList.add('language-solidity');
                c.classList.remove('language-sol');
            }

            if (lang == 'none') {
                continue;
            }

            Prism.highlightElement(c);
        }

        return () => {
            handlers.forEach(({ a, onClick }) => a.removeEventListener('click', onClick));
        };
    }, [content, ref]);

    return <div ref={ref} dangerouslySetInnerHTML={{ __html: content }} className="prose" />;
};
