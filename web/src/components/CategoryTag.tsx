import classNames from 'classnames';

import { getCategoryColor } from '@/util/category';

export const CategoryTag = ({ tag }: { tag: string }) => {
    return (
        <div
            key={tag}
            className={classNames(
                'text-sm text-white px-1 border border-primary relative bg-primary'
            )}
        >
            <div className="relative z-0 text-primary">{tag}</div>
            <div
                className="absolute top-0 left-0 w-full h-full"
                style={{
                    backgroundColor: getCategoryColor(tag),
                    opacity: 0.2,
                    backgroundBlendMode: 'soft-light',
                }}
            ></div>
        </div>
    );
};
