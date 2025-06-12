export const mapDiscourseInstanceUrl = (discourseId: string) => {
    switch (discourseId) {
        case 'magicians':
            return 'https://ethereum-magicians.org';
        case 'research':
            return 'https://ethresear.ch';
    }
};

export const mapInstanceUrlDiscourse = (url: string) => {
    switch (url) {
        case 'ethereum-magicians.org':
            return 'magicians';
        case 'ethresear.ch':
            return 'research';
    }
};
