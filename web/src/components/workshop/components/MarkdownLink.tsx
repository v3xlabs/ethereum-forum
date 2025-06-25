import { Link } from '@tanstack/react-router';
import React from 'react';

import { mapInstanceUrlDiscourse } from '@/util/discourse';

import { TopicPreviewTooltip } from './TopicPreviewTooltip';
import { UserProfileTooltip } from './UserProfileTooltip';

export const MarkdownLink = (props: React.AnchorHTMLAttributes<HTMLAnchorElement>) => {
    const { href, children, ...otherProps } = props;

    if (!href) {
        return <span {...otherProps}>{children}</span>;
    }

    // Check if it's an internal link (starts with /)
    const isInternalLink = href.startsWith('/');

    // Check if it's a user profile link (/u/instance/username) (two captures), such as /u/magicians/username where magicians and username are captured
    const userMatch = href.match(/^\/u\/([^/]+)\/([^/]+)(?:\/(\w+))?$/);
    // Check if it's a topic link (/t/topic-id or /t/slug/topic-id) on https://ethereum-magicians.org or https://ethresear.ch
    const topicMatch = href.match(
        /https:\/\/(?:www\.)?(ethereum-magicians\.org|ethresear\.ch)\/t(?:\/[\w-]+)?\/(\d+)(?:\/\d+)?/
    );

    if (userMatch) {
        const [, instance, username] = userMatch;

        return (
            <UserProfileTooltip discourseId={instance} username={username}>
                <Link
                    to="/u/$discourseId/$userId"
                    params={{ userId: username, discourseId: instance }}
                    className="text-blue-600 hover:text-blue-800 un1derline"
                    {...otherProps}
                >
                    {children}
                </Link>
            </UserProfileTooltip>
        );
    }

    if (topicMatch) {
        const [, instance, topicId] = topicMatch;

        const slug = mapInstanceUrlDiscourse(instance);

        if (!instance || !topicId || !slug) {
            return <span {...otherProps}>{children}</span>;
        }

        return (
            <TopicPreviewTooltip topicId={topicId} instance={slug}>
                <Link
                    to="/t/$discourseId/$topicId"
                    params={{
                        discourseId: slug,
                        topicId,
                    }}
                    className="text-blue-600 hover:text-blue-800 underline"
                    {...otherProps}
                >
                    {children}
                </Link>
            </TopicPreviewTooltip>
        );
    }

    if (isInternalLink) {
        return (
            <Link
                to={href as any}
                className="text-blue-600 hover:text-blue-800 underline"
                {...otherProps}
            >
                {children}
            </Link>
        );
    }

    // For external links, use regular anchor tag
    return (
        <a
            href={href}
            target="_blank"
            rel="noopener noreferrer"
            className="text-blue-600 hover:text-blue-800 underline"
            {...otherProps}
        >
            {children}
        </a>
    );
};
