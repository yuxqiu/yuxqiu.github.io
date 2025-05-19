module Jekyll
  class DraftFilter < Generator
    safe true
    priority :low

    def generate(site)
      # Check if drafts should be shown (local preview or explicit flag)
      show_drafts = site.config['show_drafts'] || ENV['JEKYLL_ENV'] == 'development' || site.config['drafts']

      # Iterate over all custom collections
      site.collections.each do |label, collection|
        # Skip the posts collection, as it has built-in draft handling
        next if label == 'posts'

        # Filter out documents with draft: true unless show_drafts is true
        unless show_drafts
          collection.docs.reject! { |doc| doc.data['draft'] == true }
        end
      end
    end
  end
end
